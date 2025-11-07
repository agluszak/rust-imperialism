use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};
use std::collections::{HashMap, HashSet};

use crate::ai::{AiControlledCivilian, AiNation};
use crate::civilians::{Civilian, CivilianKind};
use crate::constants::MAP_SIZE;
use crate::economy::{
    Allocations, Capital, Good, Name, NationColor, NationHandle, NationId, NationInstance,
    PlayerNation, RecruitmentCapacity, RecruitmentQueue, ReservationSystem, Stockpile,
    Technologies, TrainingQueue, Treasury, Workforce,
    production::{Buildings, ProductionSettings},
};
use crate::map::province::{City, Province, ProvinceId};
use crate::map::province_gen::generate_provinces;
use crate::map::tile_pos::{HexExt, TilePosExt}; // Trait methods: to_hex(), distance_to()
use crate::map::tiles::TerrainType;
use crate::resources::{DevelopmentLevel, TileResource};

/// Resource to track if provinces have been generated
#[derive(Resource)]
pub struct ProvincesGenerated;

/// Generate provinces after the tilemap is created
pub fn generate_provinces_system(
    mut commands: Commands,
    tile_storage_query: Query<&TileStorage>,
    tile_types: Query<&TerrainType>,
    provinces_generated: Option<Res<ProvincesGenerated>>,
) {
    // Skip if already generated
    if provinces_generated.is_some() {
        return;
    }

    // Wait for tile storage to exist
    let Some(tile_storage) = tile_storage_query.iter().next() else {
        return;
    };

    info!("Generating provinces...");

    let _province_entities =
        generate_provinces(&mut commands, tile_storage, &tile_types, MAP_SIZE, MAP_SIZE);

    // Cities will be spawned when provinces are assigned to countries

    commands.insert_resource(ProvincesGenerated);
    info!("Province generation complete!");
}

/// Assign provinces to countries and create capitals
pub fn assign_provinces_to_countries(
    mut commands: Commands,
    mut provinces: Query<(Entity, &mut Province)>,
    provinces_generated: Option<Res<ProvincesGenerated>>,
) {
    // Skip if provinces not yet generated
    if provinces_generated.is_none() {
        return;
    }

    // Check if already assigned (provinces have owners)
    if provinces.iter().any(|(_, p)| p.owner.is_some()) {
        return;
    }

    let province_list: Vec<(Entity, ProvinceId, TilePos)> = provinces
        .iter()
        .map(|(e, p)| (e, p.id, p.city_tile))
        .collect();

    if province_list.is_empty() {
        return;
    }

    info!(
        "Assigning {} provinces to countries...",
        province_list.len()
    );

    // Define number of countries (for now, let's say 3-5 based on province count)
    let num_countries = (province_list.len() / 8).clamp(3, 5);

    // Define distinct nation colors
    let nation_colors = [
        Color::srgb(0.2, 0.4, 0.8), // Blue
        Color::srgb(0.8, 0.2, 0.2), // Red
        Color::srgb(0.2, 0.7, 0.3), // Green
        Color::srgb(0.9, 0.7, 0.1), // Yellow
        Color::srgb(0.7, 0.2, 0.7), // Purple
    ];

    // Create countries
    let mut country_entities = Vec::new();
    let mut capitals = Vec::new();

    for i in 0..num_countries {
        let name = if i == 0 {
            "Player".to_string()
        } else {
            format!("Nation {}", i + 1)
        };

        let mut stockpile = Stockpile::default();
        if i == 0 {
            // Player starts with some resources
            // Raw materials for textile production
            stockpile.add(Good::Wool, 10);
            stockpile.add(Good::Cotton, 10);

            // Raw materials for wood/paper production
            stockpile.add(Good::Timber, 20);

            // Raw materials for steel production
            stockpile.add(Good::Coal, 10);
            stockpile.add(Good::Iron, 10);

            // Raw food for feeding workers
            stockpile.add(Good::Grain, 20);
            stockpile.add(Good::Fruit, 20);
            stockpile.add(Good::Livestock, 20);
            stockpile.add(Good::Fish, 10);

            // Finished goods for recruiting workers
            stockpile.add(Good::CannedFood, 10);
            stockpile.add(Good::Clothing, 10);
            stockpile.add(Good::Furniture, 10);

            // Paper for training workers
            stockpile.add(Good::Paper, 5);
        }

        let color = nation_colors[i % nation_colors.len()];

        let country_builder = commands.spawn((
            NationId(i as u16 + 1),
            Name(name),
            NationColor(color),
            Treasury::new(10_000),
            stockpile,
            Technologies::default(),
            Allocations::default(),       // Simplified allocation tracking
            ReservationSystem::default(), // Reservation tracking
        ));

        let country_entity = country_builder.id();

        commands.queue(move |world: &mut World| {
            if let Some(instance) = NationInstance::from_entity(world.entity(country_entity)) {
                world
                    .entity_mut(country_entity)
                    .insert(NationHandle::new(instance));
            } else {
                warn!("Failed to create NationInstance for {:?}", country_entity);
            }
        });

        if i > 0 {
            commands.entity(country_entity).insert(AiNation);
        }

        // Player gets starting buildings and workforce
        if i == 0 {
            let mut workforce = Workforce::new();
            // Start with 5 untrained workers
            workforce.add_untrained(5);
            // Sync labor pool with worker counts
            workforce.update_labor_pool();

            // All manufacturories are available at start
            commands.entity(country_entity).insert((
                Buildings::with_all_initial(),
                ProductionSettings::default(),
                workforce,
                RecruitmentCapacity::default(),
                RecruitmentQueue::default(),
                TrainingQueue::default(),
            ));

            // Note: Capitol and TradeSchool don't need separate Building entities
            // They're always available and use the nation's Stockpile/Workforce directly
        }
        country_entities.push(country_entity);
        info!("Created Nation {} with color", i + 1);
    }

    // Set player nation reference
    if let Some(&player_entity) = country_entities.first() {
        commands.queue(move |world: &mut World| {
            if let Some(player_nation) = PlayerNation::from_entity(world, player_entity) {
                world.insert_resource(player_nation);
            } else {
                warn!("Failed to initialize player nation from entity {player_entity:?}");
            }
        });
    }

    // Build adjacency map for provinces
    let adjacency_map = build_province_adjacency(&provinces);

    // Assign connected groups of provinces to countries
    let mut assigned: HashSet<ProvinceId> = HashSet::new();
    let mut country_idx = 0;

    for &(_province_entity, province_id, _city_tile) in &province_list {
        if assigned.contains(&province_id) {
            continue;
        }

        // Flood-fill to get connected provinces for this country
        let connected_group = get_connected_provinces(
            province_id,
            &adjacency_map,
            &assigned,
            province_list.len() / num_countries,
        );

        let country_entity = country_entities[country_idx % num_countries];

        // Assign all provinces in the connected group to this country
        for &prov_id in &connected_group {
            assigned.insert(prov_id);

            // Find the province entity and city tile
            if let Some(&(prov_entity, _, prov_city)) =
                province_list.iter().find(|(_, id, _)| *id == prov_id)
            {
                assign_province_to_country(
                    &mut commands,
                    &mut provinces,
                    prov_entity,
                    prov_id,
                    prov_city,
                    country_entity,
                    &mut capitals,
                );
            }
        }

        country_idx += 1;
    }

    // Handle any remaining unassigned provinces
    for (province_entity, province_id, city_tile) in province_list.iter() {
        if !assigned.contains(province_id) {
            let country_entity = country_entities[country_idx % num_countries];
            assign_province_to_country(
                &mut commands,
                &mut provinces,
                *province_entity,
                *province_id,
                *city_tile,
                country_entity,
                &mut capitals,
            );
            assigned.insert(*province_id);
            country_idx += 1;
        }
    }

    let player_entity = country_entities.first().copied();

    // Spawn starter civilian roster for the player clustered around the capital
    if let Some(player_entity) = player_entity
        && let Some(player_capital) = capitals
            .iter()
            .find(|(entity, _)| *entity == player_entity)
            .map(|(_, pos)| *pos)
    {
        let spawn_positions = gather_spawn_positions(player_capital, 6);
        let starter_units = [
            CivilianKind::Engineer,
            CivilianKind::Prospector,
            CivilianKind::Farmer,
            CivilianKind::Miner,
            CivilianKind::Rancher,
            CivilianKind::Forester,
        ];

        for (kind, pos) in starter_units.iter().zip(spawn_positions.iter()) {
            commands.spawn(Civilian {
                kind: *kind,
                position: *pos,
                owner: player_entity,
                selected: false,
                has_moved: false,
            });
            info!("Spawned {:?} for player at ({}, {})", kind, pos.x, pos.y);
        }
    }

    let ai_starter_units = [
        CivilianKind::Engineer,
        CivilianKind::Prospector,
        CivilianKind::Farmer,
        CivilianKind::Miner,
        CivilianKind::Rancher,
        CivilianKind::Forester,
    ];
    for (nation_entity, capital_pos) in capitals
        .iter()
        .copied()
        .filter(|(entity, _)| Some(*entity) != player_entity)
    {
        let spawn_positions = gather_spawn_positions(capital_pos, ai_starter_units.len());
        for (kind, pos) in ai_starter_units.iter().zip(spawn_positions.iter()) {
            commands.spawn((
                Civilian {
                    kind: *kind,
                    position: *pos,
                    owner: nation_entity,
                    selected: false,
                    has_moved: false,
                },
                AiControlledCivilian,
            ));
            info!(
                "Spawned {:?} for AI nation {:?} at ({}, {})",
                kind, nation_entity, pos.x, pos.y
            );
        }
    }

    info!("Province assignment complete!");
}

/// Assign a province to a country
fn assign_province_to_country(
    commands: &mut Commands,
    provinces: &mut Query<(Entity, &mut Province)>,
    province_entity: Entity,
    province_id: ProvinceId,
    city_tile: TilePos,
    country_entity: Entity,
    capitals: &mut Vec<(Entity, TilePos)>,
) {
    // Update province owner
    if let Ok((_, mut province)) = provinces.get_mut(province_entity) {
        province.owner = Some(country_entity);
    }

    // Create city entity
    let is_capital = !capitals.iter().any(|(entity, _)| *entity == country_entity);
    commands.spawn((
        City {
            province: province_id,
            is_capital,
        },
        city_tile,
    ));

    // If this is a capital, add Capital component to the country
    if is_capital {
        commands.entity(country_entity).insert(Capital(city_tile));
        let capital_tile = city_tile;
        commands.queue(move |world: &mut World| {
            boost_capital_food_tiles(world, capital_tile);
        });
        info!("Set capital at ({}, {})", city_tile.x, city_tile.y);
        capitals.push((country_entity, city_tile));
    }
}

fn gather_spawn_positions(capital_pos: TilePos, count: usize) -> Vec<TilePos> {
    let mut spawn_positions = Vec::new();
    spawn_positions.push(capital_pos);

    for neighbor in capital_pos.to_hex().all_neighbors() {
        if let Some(tile_pos) = neighbor.to_tile_pos() {
            spawn_positions.push(tile_pos);
        }
        if spawn_positions.len() >= count {
            break;
        }
    }

    while spawn_positions.len() < count {
        spawn_positions.push(capital_pos);
    }

    spawn_positions
}

pub(crate) fn boost_capital_food_tiles(world: &mut World, capital_pos: TilePos) {
    let mut tile_storage_query = world.query::<&TileStorage>();
    let Some(tile_storage) = tile_storage_query.iter(world).next() else {
        return;
    };

    let mut target_tiles = Vec::new();
    let mut positions = Vec::with_capacity(7);
    positions.push(capital_pos);
    for neighbor in capital_pos.to_hex().all_neighbors() {
        if let Some(tile_pos) = neighbor.to_tile_pos() {
            positions.push(tile_pos);
        }
    }

    for pos in positions {
        if let Some(tile_entity) = tile_storage.get(&pos) {
            target_tiles.push((tile_entity, pos));
        }
    }

    for (tile_entity, pos) in target_tiles {
        if let Some(mut resource) = world.get_mut::<TileResource>(tile_entity)
            && resource.discovered
            && resource.improvable_by_farmer()
            && resource.development == DevelopmentLevel::Lv0
        {
            resource.development = DevelopmentLevel::Lv1;
            debug!(
                "Auto-improved {:?} near capital at ({}, {})",
                resource.resource_type, pos.x, pos.y
            );
        }
    }
}

/// Build adjacency map for provinces based on shared tiles
fn build_province_adjacency(
    provinces: &Query<(Entity, &mut Province)>,
) -> HashMap<ProvinceId, Vec<ProvinceId>> {
    use std::collections::{HashMap, HashSet};

    let mut adjacency: HashMap<ProvinceId, HashSet<ProvinceId>> = HashMap::new();

    // Collect all province tiles
    let province_tiles: Vec<(ProvinceId, Vec<TilePos>)> = provinces
        .iter()
        .map(|(_, p)| (p.id, p.tiles.clone()))
        .collect();

    // Check each province against all others
    for (i, (id1, tiles1)) in province_tiles.iter().enumerate() {
        for (id2, tiles2) in province_tiles.iter().skip(i + 1) {
            // Check if any tiles are adjacent
            let mut are_adjacent = false;
            'outer: for tile1 in tiles1 {
                let hex1 = tile1.to_hex();
                for tile2 in tiles2 {
                    let hex2 = tile2.to_hex();
                    if hex1.distance_to(hex2) == 1 {
                        are_adjacent = true;
                        break 'outer;
                    }
                }
            }

            if are_adjacent {
                adjacency.entry(*id1).or_default().insert(*id2);
                adjacency.entry(*id2).or_default().insert(*id1);
            }
        }
    }

    // Convert to Vec for easier iteration
    adjacency
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

/// Get connected provinces using flood-fill
fn get_connected_provinces(
    start: ProvinceId,
    adjacency: &HashMap<ProvinceId, Vec<ProvinceId>>,
    already_assigned: &HashSet<ProvinceId>,
    target_size: usize,
) -> Vec<ProvinceId> {
    use std::collections::{HashSet, VecDeque};

    let mut connected = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        if already_assigned.contains(&current) {
            continue;
        }

        connected.push(current);

        // Stop if we've reached target size
        if connected.len() >= target_size {
            break;
        }

        // Add unvisited neighbors
        if let Some(neighbors) = adjacency.get(&current) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) && !already_assigned.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
    }

    connected
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use crate::ai::{AiControlledCivilian, AiNation};
    use crate::civilians::Civilian;
    use crate::map::province::{Province, ProvinceId};
    use crate::map::province_setup::{
        ProvincesGenerated, assign_provinces_to_countries, boost_capital_food_tiles,
    };
    use crate::resources::{DevelopmentLevel, ResourceType, TileResource};

    #[test]
    fn capital_adjacent_food_tiles_start_at_level_one() {
        let mut world = World::new();

        let mut tile_storage = TileStorage::empty(TilemapSize { x: 3, y: 3 });
        let capital_pos = TilePos { x: 1, y: 1 };
        let capital_tile = world.spawn(TileResource::visible(ResourceType::Grain)).id();
        tile_storage.set(&capital_pos, capital_tile);

        let neighbor_pos = TilePos { x: 1, y: 2 };
        let neighbor_tile = world
            .spawn(TileResource::visible(ResourceType::Cotton))
            .id();
        tile_storage.set(&neighbor_pos, neighbor_tile);

        let mineral_pos = TilePos { x: 0, y: 0 };
        let mineral_tile = world
            .spawn(TileResource::hidden_mineral(ResourceType::Coal))
            .id();
        tile_storage.set(&mineral_pos, mineral_tile);

        world.spawn(tile_storage);

        boost_capital_food_tiles(&mut world, capital_pos);

        let capital_resource = world
            .get::<TileResource>(capital_tile)
            .expect("capital tile should have resource");
        assert_eq!(capital_resource.development, DevelopmentLevel::Lv1);

        let neighbor_resource = world
            .get::<TileResource>(neighbor_tile)
            .expect("neighbor tile should have resource");
        assert_eq!(neighbor_resource.development, DevelopmentLevel::Lv1);

        let mineral_resource = world
            .get::<TileResource>(mineral_tile)
            .expect("mineral tile should have resource");
        assert_eq!(mineral_resource.development, DevelopmentLevel::Lv0);
        assert!(!mineral_resource.discovered);
    }

    #[test]
    fn ai_nations_receive_capitals_and_civilians() {
        let mut world = World::new();
        world.insert_resource(ProvincesGenerated);

        let province_positions = [
            TilePos { x: 0, y: 0 },
            TilePos { x: 1, y: 0 },
            TilePos { x: 2, y: 0 },
            TilePos { x: 3, y: 0 },
            TilePos { x: 4, y: 0 },
            TilePos { x: 5, y: 0 },
        ];

        for (index, position) in province_positions.iter().enumerate() {
            world.spawn(Province::new(
                ProvinceId(index as u32),
                vec![*position],
                *position,
            ));
        }

        let _ = world.run_system_once(assign_provinces_to_countries);
        world.flush();

        let mut ai_nation_query = world.query_filtered::<Entity, With<AiNation>>();
        let ai_nations: Vec<Entity> = ai_nation_query.iter(&world).collect();
        assert!(
            ai_nations.len() >= 1,
            "expected at least one AI nation to be created"
        );

        let mut ai_civilian_query =
            world.query_filtered::<(Entity, &Civilian), With<AiControlledCivilian>>();
        assert!(
            ai_civilian_query.iter(&world).count() >= ai_nations.len(),
            "expected each AI nation to spawn civilians"
        );

        for nation in ai_nations {
            let owned_units: Vec<Entity> = ai_civilian_query
                .iter(&world)
                .filter(|(_, civilian)| civilian.owner == nation)
                .map(|(entity, _)| entity)
                .collect();
            assert!(
                !owned_units.is_empty(),
                "AI nation {:?} should have at least one controlled civilian",
                nation
            );
        }
    }
}
