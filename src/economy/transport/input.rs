use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use crate::economy::transport::messages::PlaceImprovement;
use crate::economy::transport::types::{
    Depot, ImprovementKind, Port, RailConstruction, Rails, Roads, ordered_edge,
};
use crate::economy::transport::validation::{are_adjacent, can_build_rail_on_terrain};
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::map::tiles::TerrainType;

use crate::economy::{nation::PlayerNation, technology::Technologies, treasury::Treasury};

/// Apply improvement placements (Input Layer)
/// Reads PlaceImprovement messages, validates, charges treasury, spawns entities
pub fn apply_improvements(
    mut commands: Commands,
    mut ev: MessageReader<PlaceImprovement>,
    mut roads: ResMut<Roads>,
    rails: ResMut<Rails>,
    player: Option<Res<PlayerNation>>,
    mut treasuries: Query<&mut Treasury>,
    nations: Query<&Technologies>,
    tile_storage_query: Query<&TileStorage>,
    tile_types: Query<&TerrainType>,
) {
    for e in ev.read() {
        match e.kind {
            ImprovementKind::Road => {
                handle_road_placement(e.a, e.b, &mut roads, &player, &mut treasuries);
            }
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
}

fn handle_road_placement(
    a: TilePos,
    b: TilePos,
    roads: &mut ResMut<Roads>,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
) {
    if !are_adjacent(a, b) {
        return;
    }
    let edge = ordered_edge(a, b);
    // Toggle behavior: if road exists, remove for free; otherwise place with cost
    if roads.0.contains(&edge) {
        roads.0.remove(&edge);
        info!(
            "Removed road between ({}, {}) and ({}, {})",
            edge.0.x, edge.0.y, edge.1.x, edge.1.y
        );
    } else {
        let cost: i64 = 10;
        if let Some(player) = &player
            && let Ok(mut treasury) = treasuries.get_mut(player.entity())
        {
            if treasury.total() >= cost {
                treasury.subtract(cost);
                roads.0.insert(edge);
                info!(
                    "Built road between ({}, {}) and ({}, {}) for ${}",
                    edge.0.x, edge.0.y, edge.1.x, edge.1.y, cost
                );
            } else {
                info!(
                    "Not enough money to build road (need ${}, have ${})",
                    cost,
                    treasury.total()
                );
            }
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
            commands.spawn(RailConstruction {
                from: edge.0,
                to: edge.1,
                turns_remaining: 2,
                owner: nation_entity,
                engineer: e.engineer.unwrap_or(nation_entity),
            });

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
            commands.spawn(Depot {
                position: a,
                owner: owner_entity,
                connected: false, // Will be computed by connectivity system
            });
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
    let hex = port_pos.to_hex();

    // Check if any adjacent tile is water
    let mut adjacent_to_water = false;
    for tile_storage in tile_storage_query.iter() {
        for neighbor_hex in hex.all_neighbors() {
            if let Some(neighbor_pos) = neighbor_hex.to_tile_pos()
                && let Some(neighbor_entity) = tile_storage.get(&neighbor_pos)
                && let Ok(terrain) = tile_types.get(neighbor_entity)
                && *terrain == TerrainType::Water
            {
                adjacent_to_water = true;
                break;
            }
        }
        if adjacent_to_water {
            break;
        }
    }

    if !adjacent_to_water {
        info!(
            "Cannot build port at ({}, {}): must be adjacent to water",
            port_pos.x, port_pos.y
        );
        return;
    }

    // Port is placed on a single tile
    let cost: i64 = 150;

    // Determine owner: prefer explicit nation, fallback to player
    let owner = nation.or_else(|| player.as_ref().map(|p| p.entity()));

    if let Some(owner_entity) = owner
        && let Ok(mut treasury) = treasuries.get_mut(owner_entity)
    {
        if treasury.total() >= cost {
            treasury.subtract(cost);
            commands.spawn(Port {
                position: a,
                owner: owner_entity,
                connected: false,
                is_river: false, // TODO: detect from terrain
            });
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
