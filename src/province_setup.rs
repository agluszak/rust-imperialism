use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use crate::constants::MAP_SIZE;
use crate::province::{City, Province, ProvinceId};
use crate::province_gen::generate_provinces;
use crate::tile_pos::TilePosExt; // HexExt used for trait methods: to_hex(), distance_to()
use crate::tiles::TerrainType;

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
    let num_countries = ((province_list.len() / 8).max(3)).min(5);

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
    for i in 0..num_countries {
        let name = if i == 0 {
            "Player".to_string()
        } else {
            format!("Nation {}", i + 1)
        };

        let mut stockpile = crate::economy::Stockpile::default();
        if i == 0 {
            // Player starts with some resources
            stockpile.add(crate::economy::Good::Wool, 10);
            stockpile.add(crate::economy::Good::Cotton, 10);

            // Raw food for feeding workers
            stockpile.add(crate::economy::Good::Grain, 20);
            stockpile.add(crate::economy::Good::Fruit, 20);
            stockpile.add(crate::economy::Good::Livestock, 20);

            // Finished goods for recruiting workers
            stockpile.add(crate::economy::Good::CannedFood, 10);
            stockpile.add(crate::economy::Good::Clothing, 10);
            stockpile.add(crate::economy::Good::Furniture, 10);

            // Paper for training workers
            stockpile.add(crate::economy::Good::Paper, 5);
        }

        let color = nation_colors[i % nation_colors.len()];

        let country_builder = commands.spawn((
            crate::economy::NationId(i as u16 + 1),
            crate::economy::Name(name),
            crate::economy::NationColor(color),
            crate::economy::Treasury::new(10_000),
            stockpile,
            crate::economy::Technologies::default(),
            crate::economy::Allocations::default(), // Simplified allocation tracking
            crate::economy::ReservationSystem::default(), // Reservation tracking
        ));

        let country_entity = country_builder.id();

        // Player gets starting buildings and workforce
        if i == 0 {
            let mut workforce = crate::economy::Workforce::new();
            // Start with 5 untrained workers
            workforce.add_untrained(5);

            // Textile mill is the main production building on the nation entity
            commands.entity(country_entity).insert((
                crate::economy::Building::textile_mill(8), // Capacity of 8
                crate::economy::production::ProductionSettings::default(),
                workforce,
                crate::economy::RecruitmentCapacity::default(),
                crate::economy::RecruitmentQueue::default(),
                crate::economy::TrainingQueue::default(),
            ));

            // Note: Capitol and TradeSchool don't need separate Building entities
            // They're always available and use the nation's Stockpile/Workforce directly
        }
        country_entities.push(country_entity);
        info!("Created Nation {} with color", i + 1);
    }

    // Set player nation reference
    if !country_entities.is_empty() {
        commands.insert_resource(crate::economy::PlayerNation(country_entities[0]));
    }

    // Build adjacency map for provinces
    let adjacency_map = build_province_adjacency(&provinces);

    // Assign connected groups of provinces to countries
    let mut assigned: std::collections::HashSet<ProvinceId> = std::collections::HashSet::new();
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
                    country_idx == 0, // First country gets first province as capital
                    &assigned,
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
                false,
                &assigned,
            );
            assigned.insert(*province_id);
            country_idx += 1;
        }
    }

    // Spawn an Engineer for the player near their capital
    if let Some(player_entity) = country_entities.first()
        && let Some(player_capital) = province_list.first()
    {
        let engineer_pos = player_capital.2; // Use capital tile for now
        commands.spawn(crate::civilians::Civilian {
            kind: crate::civilians::CivilianKind::Engineer,
            position: engineer_pos,
            owner: *player_entity,
            selected: false,
            has_moved: false,
        });
        info!(
            "Spawned Engineer for player at ({}, {})",
            engineer_pos.x, engineer_pos.y
        );
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
    is_first_of_country: bool,
    assigned: &std::collections::HashSet<ProvinceId>,
) {
    // Update province owner
    if let Ok((_, mut province)) = provinces.get_mut(province_entity) {
        province.owner = Some(country_entity);
    }

    // Create city entity
    let is_capital = is_first_of_country && assigned.len() == 1;
    commands.spawn((
        City {
            province: province_id,
            is_capital,
        },
        city_tile,
    ));

    // If this is a capital, add Capital component to the country
    if is_capital {
        commands
            .entity(country_entity)
            .insert(crate::economy::Capital(city_tile));
        info!("Set capital at ({}, {})", city_tile.x, city_tile.y);
    }
}

/// Build adjacency map for provinces based on shared tiles
fn build_province_adjacency(
    provinces: &Query<(Entity, &mut Province)>,
) -> std::collections::HashMap<ProvinceId, Vec<ProvinceId>> {
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
    adjacency: &std::collections::HashMap<ProvinceId, Vec<ProvinceId>>,
    already_assigned: &std::collections::HashSet<ProvinceId>,
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
