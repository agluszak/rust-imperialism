use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};
use std::collections::{HashMap, HashSet};

use crate::ai::{AiControlledCivilian, AiNation};
use crate::civilians::{Civilian, CivilianKind};
use crate::constants::MAP_SIZE;
use crate::economy::Rails;
use crate::economy::{
    Allocations, Capital, Good, Nation, NationColor, OwnedBy, PlayerNation, RecruitmentCapacity,
    RecruitmentQueue, ReservationSystem, Stockpile, Technologies, TrainingQueue, Treasury,
    Workforce,
    production::{Buildings, ProductionSettings},
};
use crate::map::province::{City, Province, ProvinceId};
use crate::map::province_gen::generate_provinces;
use crate::map::rendering::{BorderLine, MapVisualFor};
use crate::map::tile_pos::{HexExt, TilePosExt}; // Trait methods: to_hex(), distance_to()
use crate::map::tiles::TerrainType;
use crate::resources::{DevelopmentLevel, TileResource};
use crate::ui::components::MapTilemap;

/// Resource to enable map pruning for tests
#[derive(Resource, Default)]
pub struct TestMapConfig;

/// Generate provinces after the tilemap is created
pub fn generate_provinces_system(
    mut commands: Commands,
    tile_storage_query: Query<&TileStorage>,
    tile_types: Query<&TerrainType>,
) {
    // Wait for tile storage to exist
    let Some(tile_storage) = tile_storage_query.iter().next() else {
        return;
    };

    info!("Generating provinces...");

    let _province_entities =
        generate_provinces(&mut commands, tile_storage, &tile_types, MAP_SIZE, MAP_SIZE);

    // Cities will be spawned when provinces are assigned to countries
    info!("Province generation complete!");
}

/// Assign provinces to countries and create capitals
pub fn assign_provinces_to_countries(
    mut commands: Commands,
    mut provinces: Query<(Entity, &mut Province)>,
    mut next_civilian_id: ResMut<crate::civilians::types::NextCivilianId>,
) {
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
    let mut country_entities: Vec<Entity> = Vec::new();
    let mut capitals = Vec::new();

    for i in 0..num_countries {
        let color = nation_colors[i % nation_colors.len()];
        let color_name = match i % nation_colors.len() {
            0 => "Blue",
            1 => "Red",
            2 => "Green",
            3 => "Yellow",
            4 => "Purple",
            _ => "Unknown",
        };

        let name = if i == 0 {
            format!("Player ({})", color_name)
        } else {
            format!("Nation {}", color_name)
        };

        let stockpile = baseline_stockpile();

        let country_builder = commands.spawn((
            Nation,
            Name::new(name),
            NationColor(color),
            Treasury::new(10_000),
            stockpile,
            Technologies::default(),
            Allocations::default(),       // Simplified allocation tracking
            ReservationSystem::default(), // Reservation tracking
        ));

        let country_entity = country_builder.id();

        if i > 0 {
            commands.entity(country_entity).insert(AiNation);
        }

        // Give every nation a basic industrial base so AI economies can function
        let mut workforce = Workforce::new();
        let starting_workers = if i == 0 { 5 } else { 3 };
        workforce.add_untrained(starting_workers);
        workforce.update_labor_pool();

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
            let civilian_id = next_civilian_id.next_id();
            let name = format!("{:?} {}", kind, civilian_id.0);
            commands.spawn((
                Civilian {
                    kind: *kind,
                    position: *pos,
                    owner: player_entity,
                    civilian_id,
                    has_moved: false,
                },
                OwnedBy(player_entity),
                Name::new(name.clone()),
            ));
            info!("Spawned {} for player at ({}, {})", name, pos.x, pos.y);
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
            let civilian_id = next_civilian_id.next_id();
            let name = format!("{:?} {}", kind, civilian_id.0);
            commands.spawn((
                Civilian {
                    kind: *kind,
                    position: *pos,
                    owner: nation_entity,
                    civilian_id,
                    has_moved: false,
                },
                AiControlledCivilian,
                OwnedBy(nation_entity),
                Name::new(name.clone()),
            ));
            info!(
                "Spawned {} for AI nation {:?} at ({}, {})",
                name, nation_entity, pos.x, pos.y
            );
        }
    }

    info!("Province assignment complete!");
}

/// Prune the map to only include the Red nation's territory for tests
pub fn prune_to_test_map(
    mut commands: Commands,
    test_config: Option<Res<TestMapConfig>>,
    nations: Query<(Entity, &NationColor)>,
    owned_entities: Query<(Entity, &OwnedBy)>,
    provinces: Query<(Entity, &Province)>,
    tiles: Query<(Entity, &TilePos)>,
    mut tile_storage_query: Query<(&mut TileStorage, &TilemapSize)>,
    border_lines: Query<Entity, With<BorderLine>>,
    visuals: Query<(Entity, &MapVisualFor)>,
    floating_visuals: Query<
        Entity,
        (
            With<MapTilemap>,
            Without<TilePos>,
            Without<TilemapSize>,
            Without<MapVisualFor>,
        ),
    >,
    rails: Option<ResMut<Rails>>,
) {
    // Only run if TestMapConfig is present
    if test_config.is_none() {
        return;
    }

    info!("Pruning map to Red nation territory using relationships...");

    // 1. Find the Red nation
    let red_color = Color::srgb(0.8, 0.2, 0.2);
    let mut red_nation_entity = None;

    for (entity, color) in nations.iter() {
        let linear = color.0.to_linear();
        if (linear.red - red_color.to_linear().red).abs() < 0.01
            && (linear.green - red_color.to_linear().green).abs() < 0.01
            && (linear.blue - red_color.to_linear().blue).abs() < 0.01
        {
            red_nation_entity = Some(entity);
            break;
        }
    }

    let Some(red_nation) = red_nation_entity else {
        warn!("Red nation not found for pruning!");
        return;
    };

    // 2. Identify all entities to keep
    let mut entities_to_keep = std::collections::HashSet::new();
    entities_to_keep.insert(red_nation);

    let mut tile_positions_to_keep = std::collections::HashSet::new();

    // Provinces and their tiles
    for (entity, province) in provinces.iter() {
        if province.owner == Some(red_nation) {
            entities_to_keep.insert(entity);
            for pos in &province.tiles {
                tile_positions_to_keep.insert(*pos);
            }
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Other owned entities (civilians, ships, cities, depots, etc.)
    for (entity, owned_by) in owned_entities.iter() {
        if owned_by.0 == red_nation {
            entities_to_keep.insert(entity);
        }
    }

    // 3. Despawn non-kept entities
    // Nations
    for (entity, _) in nations.iter() {
        if !entities_to_keep.contains(&entity) {
            commands.entity(entity).despawn();
        }
    }

    // Owned entities (Provinces, Cities, Civilians, Ships, Buildings)
    for (entity, owned_by) in owned_entities.iter() {
        if owned_by.0 != red_nation {
            // Note: Province entities were also collected here.
            // City entities were also collected here.
            commands.entity(entity).despawn();
        }
    }

    // 4. Despawn tiles and update storage
    for (mut tile_storage, _) in tile_storage_query.iter_mut() {
        let storage: &mut TileStorage = &mut tile_storage;
        for (entity, pos) in tiles.iter() {
            if !tile_positions_to_keep.contains(pos) {
                commands.entity(entity).despawn();
                storage.remove(pos);
            }
        }
    }

    // 5. Cleanup Visuals
    // Visuals linked to entities
    for (visual_entity, visual_for) in visuals.iter() {
        if !entities_to_keep.contains(&visual_for.0) {
            commands.entity(visual_entity).despawn();
        }
    }

    // Floating visuals (UI, markers that are NOT linked to an entity)
    for entity in floating_visuals.iter() {
        commands.entity(entity).despawn();
    }

    // Border lines (always reset)
    for entity in border_lines.iter() {
        commands.entity(entity).despawn();
    }

    // 6. Prune Rails resources
    if let Some(mut rails) = rails {
        rails.0.retain(|(a, b)| {
            tile_positions_to_keep.contains(a) && tile_positions_to_keep.contains(b)
        });
    }

    // Remove the config so it only runs once
    commands.remove_resource::<TestMapConfig>();
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
    if let Ok((province_entity, mut province)) = provinces.get_mut(province_entity) {
        province.owner = Some(country_entity);
        commands
            .entity(province_entity)
            .insert(OwnedBy(country_entity));
    }

    // Create city entity
    let is_capital = !capitals.iter().any(|(entity, _)| *entity == country_entity);
    commands.spawn((
        City {
            province: province_id,
            is_capital,
        },
        city_tile,
        OwnedBy(country_entity),
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

fn baseline_stockpile() -> Stockpile {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Wool, 10);
    stockpile.add(Good::Cotton, 10);
    stockpile.add(Good::Timber, 20);
    stockpile.add(Good::Coal, 10);
    stockpile.add(Good::Iron, 10);
    stockpile.add(Good::Grain, 20);
    stockpile.add(Good::Fruit, 20);
    stockpile.add(Good::Livestock, 20);
    stockpile.add(Good::Fish, 10);
    stockpile.add(Good::CannedFood, 10);
    stockpile.add(Good::Clothing, 10);
    stockpile.add(Good::Furniture, 10);
    stockpile.add(Good::Paper, 5);
    stockpile
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
    // Collect all province tiles
    let province_tiles: Vec<(ProvinceId, Vec<TilePos>)> = provinces
        .iter()
        .map(|(_, p)| (p.id, p.tiles.clone()))
        .collect();

    calculate_adjacency(&province_tiles)
}

pub fn calculate_adjacency(
    province_tiles: &[(ProvinceId, Vec<TilePos>)],
) -> HashMap<ProvinceId, Vec<ProvinceId>> {
    use std::collections::{HashMap, HashSet};

    let mut adjacency: HashMap<ProvinceId, HashSet<ProvinceId>> = HashMap::new();
    let mut tile_to_province: HashMap<TilePos, ProvinceId> = HashMap::new();

    // 1. Build tile -> province map
    for (province_id, tiles) in province_tiles {
        for tile_pos in tiles {
            tile_to_province.insert(*tile_pos, *province_id);
        }
    }

    // 2. Check neighbors
    for (province_id, tiles) in province_tiles {
        for tile_pos in tiles {
            let hex = tile_pos.to_hex();
            for neighbor_hex in hex.all_neighbors() {
                // If neighbor is on the map
                if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                    // Check if neighbor belongs to a province
                    if let Some(neighbor_province) = tile_to_province.get(&neighbor_pos) {
                        // If it belongs to a different province, record adjacency
                        if neighbor_province != province_id {
                            adjacency
                                .entry(*province_id)
                                .or_default()
                                .insert(*neighbor_province);
                            // We don't strictly need to insert the reverse here because
                            // we will eventually visit the neighbor tile and insert the reverse then.
                            // But doing it here ensures symmetry even if tiles are processed weirdly,
                            // and HashSet handles duplicates cheaply.
                            adjacency
                                .entry(*neighbor_province)
                                .or_default()
                                .insert(*province_id);
                        }
                    }
                }
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
    use crate::map::province_setup::{assign_provinces_to_countries, boost_capital_food_tiles};
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
        // Removed ProvincesGenerated resource insertion
        world.insert_resource(crate::civilians::types::NextCivilianId::default());

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
            !ai_nations.is_empty(),
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
