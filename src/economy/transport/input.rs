use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use crate::economy::transport::messages::PlaceImprovement;
use crate::economy::transport::types::{
    Depot, ImprovementKind, Port, RailConstruction, Rails, ordered_edge,
};
use crate::economy::transport::validation::{are_adjacent, can_build_rail_on_terrain};
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::map::tiles::TerrainType;
use hexx::Hex;

use crate::economy::{
    nation::{OwnedBy, PlayerNation},
    technology::Technologies,
    treasury::Treasury,
};

/// Apply improvement placements (Input Layer)
/// Observer triggered by PlaceImprovement events, validates, charges treasury, spawns entities
pub fn apply_improvements(
    trigger: On<PlaceImprovement>,
    mut commands: Commands,
    rails: ResMut<Rails>,
    player: Option<Res<PlayerNation>>,
    mut treasuries: Query<&mut Treasury>,
    nations: Query<&Technologies>,
    tile_storage_query: Query<&TileStorage>,
    tile_types: Query<&TerrainType>,
) {
    let e = trigger.event();
    match e.kind {
        ImprovementKind::Rail => {
            handle_rail_construction(
                &mut commands,
                e,
                &rails,
                &player,
                &mut treasuries,
                &nations,
                &tile_storage_query,
                &tile_types,
            );
        }
        ImprovementKind::Depot => {
            handle_depot_placement(&mut commands, e.a, e.nation, &player, &mut treasuries);
        }
        ImprovementKind::Port => {
            handle_port_placement(
                &mut commands,
                e.a,
                e.nation,
                &player,
                &mut treasuries,
                &tile_storage_query,
                &tile_types,
            );
        }
    }
}

fn handle_rail_construction(
    commands: &mut Commands,
    e: &PlaceImprovement,
    rails: &ResMut<Rails>,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
    nations: &Query<&Technologies>,
    tile_storage_query: &Query<&TileStorage>,
    tile_types: &Query<&TerrainType>,
) {
    if !are_adjacent(e.a, e.b) {
        return;
    }
    let edge = ordered_edge(e.a, e.b);

    // Check if rail already exists
    if rails.0.contains(&edge) {
        info!(
            "Rail already exists between ({}, {}) and ({}, {})",
            edge.0.x, edge.0.y, edge.1.x, edge.1.y
        );
        return;
    }

    // Check terrain buildability for both endpoints
    // Determine builder nation (AI or Player)
    let builder_nation = e.nation.or_else(|| player.as_ref().map(|p| p.entity()));

    if let Some(nation_entity) = builder_nation {
        // Get builder nation's technologies
        let builder_techs = nations.get(nation_entity).ok();

        // Check both tiles for terrain restrictions
        let mut can_build = true;
        let mut failure_reason: Option<String> = None;

        for tile_storage in tile_storage_query.iter() {
            // Check tile A - if we can't find it, fail the build
            match tile_storage.get(&e.a) {
                Some(tile_entity_a) => {
                    if let Ok(terrain_a) = tile_types.get(tile_entity_a)
                        && let Some(techs) = builder_techs
                    {
                        let (buildable, reason) = can_build_rail_on_terrain(terrain_a, techs);
                        if !buildable {
                            can_build = false;
                            failure_reason = Some(format!(
                                "Cannot build at ({}, {}): {}",
                                e.a.x,
                                e.a.y,
                                reason.unwrap_or("terrain restriction")
                            ));
                            break;
                        }
                    }
                }
                None => {
                    // Tile not found in storage - treat as unbuildable
                    can_build = false;
                    failure_reason = Some(format!("Tile ({}, {}) not found", e.a.x, e.a.y));
                    break;
                }
            }

            // Check tile B - if we can't find it, fail the build
            match tile_storage.get(&e.b) {
                Some(tile_entity_b) => {
                    if let Ok(terrain_b) = tile_types.get(tile_entity_b)
                        && let Some(techs) = builder_techs
                    {
                        let (buildable, reason) = can_build_rail_on_terrain(terrain_b, techs);
                        if !buildable {
                            can_build = false;
                            failure_reason = Some(format!(
                                "Cannot build at ({}, {}): {}",
                                e.b.x,
                                e.b.y,
                                reason.unwrap_or("terrain restriction")
                            ));
                            break;
                        }
                    }
                }
                None => {
                    // Tile not found in storage - treat as unbuildable
                    can_build = false;
                    failure_reason = Some(format!("Tile ({}, {}) not found", e.b.x, e.b.y));
                    break;
                }
            }
        }

        if !can_build {
            info!(
                "{}",
                failure_reason.unwrap_or_else(|| "Cannot build rail on this terrain".to_string())
            );
            return;
        }
    }

    // Start rail construction (takes 2 turns)
    let cost: i64 = 50;
    if let Some(nation_entity) = builder_nation
        && let Ok(mut treasury) = treasuries.get_mut(nation_entity)
    {
        if treasury.total() >= cost {
            treasury.subtract(cost);
            commands.spawn((
                RailConstruction {
                    from: edge.0,
                    to: edge.1,
                    turns_remaining: 2,
                    owner: nation_entity,
                    engineer: e.engineer.unwrap_or(nation_entity),
                },
                OwnedBy(nation_entity),
            ));

            info!(
                "Started rail construction from ({}, {}) to ({}, {}) for ${} (2 turns)",
                edge.0.x, edge.0.y, edge.1.x, edge.1.y, cost
            );
        } else {
            info!(
                "Not enough money to build rail (need ${}, have ${})",
                cost,
                treasury.total()
            );
        }
    }
}

fn handle_depot_placement(
    commands: &mut Commands,
    a: TilePos,
    nation: Option<Entity>,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
) {
    // Depot is placed on a single tile (use position 'a', ignore 'b')
    let cost: i64 = 100;

    // Determine owner: prefer explicit nation, fallback to player
    let owner = nation.or_else(|| player.as_ref().map(|p| p.entity()));

    if let Some(owner_entity) = owner
        && let Ok(mut treasury) = treasuries.get_mut(owner_entity)
    {
        if treasury.total() >= cost {
            treasury.subtract(cost);
            commands.spawn((
                Depot {
                    position: a,
                    owner: owner_entity,
                    connected: false, // Will be computed by connectivity system
                },
                OwnedBy(owner_entity),
            ));
            info!("Built depot at ({}, {}) for ${}", a.x, a.y, cost);
        } else {
            info!(
                "Not enough money to build depot (need ${}, have ${})",
                cost,
                treasury.total()
            );
        }
    }
}

fn handle_port_placement(
    commands: &mut Commands,
    a: TilePos,
    nation: Option<Entity>,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
    tile_storage_query: &Query<&TileStorage>,
    tile_types: &Query<&TerrainType>,
) {
    // Port must be adjacent to water
    let port_pos = a;
    let adjacent_water = find_adjacent_water_tiles(port_pos, tile_storage_query, tile_types);
    let Some((storage, adjacent_water_tiles)) = adjacent_water else {
        info!(
            "Cannot build port at ({}, {}): must be adjacent to water",
            port_pos.x, port_pos.y
        );
        return;
    };

    // River port: all adjacent water tiles are rivers.
    // Ocean port: at least one adjacent water tile is ocean.
    let is_river = adjacent_water_tiles
        .iter()
        .all(|&water_pos| !is_ocean_tile(water_pos, storage, tile_types));

    // Port is placed on a single tile
    let cost: i64 = 150;

    // Determine owner: prefer explicit nation, fallback to player
    let owner = nation.or_else(|| player.as_ref().map(|p| p.entity()));

    if let Some(owner_entity) = owner
        && let Ok(mut treasury) = treasuries.get_mut(owner_entity)
    {
        if treasury.total() >= cost {
            treasury.subtract(cost);
            commands.spawn((
                Port {
                    position: a,
                    owner: owner_entity,
                    connected: false,
                    is_river,
                },
                OwnedBy(owner_entity),
            ));
            info!("Built port at ({}, {}) for ${}", a.x, a.y, cost);
        } else {
            info!(
                "Not enough money to build port (need ${}, have ${})",
                cost,
                treasury.total()
            );
        }
    }
}

fn find_adjacent_water_tiles<'a>(
    center: TilePos,
    tile_storage_query: &'a Query<&TileStorage>,
    tile_types: &Query<&TerrainType>,
) -> Option<(&'a TileStorage, Vec<TilePos>)> {
    let center_hex = center.to_hex();
    for tile_storage in tile_storage_query.iter() {
        let water_tiles: Vec<TilePos> = center_hex
            .all_neighbors()
            .into_iter()
            .filter_map(|neighbor_hex| neighbor_hex.to_tile_pos())
            .filter(|neighbor_pos| {
                tile_storage
                    .get(neighbor_pos)
                    .and_then(|neighbor_entity| tile_types.get(neighbor_entity).ok())
                    .is_some_and(|terrain| *terrain == TerrainType::Water)
            })
            .collect();

        if !water_tiles.is_empty() {
            return Some((tile_storage, water_tiles));
        }
    }
    None
}

/// Helper function to classify a water tile as Ocean or River
/// Returns true if the tile is part of an Ocean (open water or coast)
/// Returns false if the tile is part of a River (narrow channel or confluence)
pub(crate) fn is_ocean_tile(
    pos: TilePos,
    tile_storage: &TileStorage,
    tile_types: &Query<&TerrainType>,
) -> bool {
    let hex = pos.to_hex();

    // Use specific offsets to ensure we iterate neighbors in a contiguous ring
    // Axial coordinates offsets: (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)
    let offsets = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

    let mut water_neighbors = 0;
    let mut neighbor_is_water = [false; 6];

    for (i, (dx, dy)) in offsets.iter().enumerate() {
        let neighbor_hex = Hex::new(hex.x + dx, hex.y + dy);

        // Check if neighbor is water
        // Note: Out of bounds (None) is treated as Land (not Water)
        if let Some(n_pos) = neighbor_hex.to_tile_pos()
            && let Some(n_entity) = tile_storage.get(&n_pos)
            && let Ok(terrain) = tile_types.get(n_entity)
            && *terrain == TerrainType::Water
        {
            neighbor_is_water[i] = true;
            water_neighbors += 1;
        }
    }

    // Logic to distinguish Ocean vs River:
    // 1. If surrounded by Water (>= 4 neighbors), it's Open Water/Ocean.
    if water_neighbors >= 4 {
        return true;
    }

    // 2. If surrounded by Land (<= 2 neighbors), it's a River/Canal/Lake-end.
    if water_neighbors <= 2 {
        return false;
    }

    // 3. If exactly 3 neighbors are water:
    //    - If they are contiguous, it's a Straight Coast (Ocean).
    //    - If they are separated, it's a River Confluence.
    // Count transitions from Water to Land and Land to Water
    let mut transitions = 0;
    for i in 0..6 {
        let current = neighbor_is_water[i];
        let next = neighbor_is_water[(i + 1) % 6];
        if current != next {
            transitions += 1;
        }
    }

    // Coast: WWWLLL (2 transitions)
    // Confluence: WLWLWL (6 transitions)
    // Or WLWWLL (4 transitions)
    // Coast implies contiguous block of water.
    transitions <= 2
}
