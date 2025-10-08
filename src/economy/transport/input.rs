use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use super::messages::PlaceImprovement;
use super::types::{Depot, ImprovementKind, Port, RailConstruction, Rails, Roads, ordered_edge};
use super::validation::{are_adjacent, can_build_rail_on_terrain};
use crate::tile_pos::{HexExt, TilePosExt};
use crate::tiles::TerrainType;
use crate::ui::logging::TerminalLogEvent;

use super::super::{nation::PlayerNation, technology::Technologies, treasury::Treasury};

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
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for e in ev.read() {
        match e.kind {
            ImprovementKind::Road => {
                handle_road_placement(
                    e.a,
                    e.b,
                    &mut roads,
                    &player,
                    &mut treasuries,
                    &mut log_events,
                );
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
                    &mut log_events,
                );
            }
            ImprovementKind::Depot => {
                handle_depot_placement(
                    &mut commands,
                    e.a,
                    &player,
                    &mut treasuries,
                    &mut log_events,
                );
            }
            ImprovementKind::Port => {
                handle_port_placement(
                    &mut commands,
                    e.a,
                    &player,
                    &mut treasuries,
                    &tile_storage_query,
                    &tile_types,
                    &mut log_events,
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
    log_events: &mut MessageWriter<TerminalLogEvent>,
) {
    if !are_adjacent(a, b) {
        return;
    }
    let edge = ordered_edge(a, b);
    // Toggle behavior: if road exists, remove for free; otherwise place with cost
    if roads.0.contains(&edge) {
        roads.0.remove(&edge);
        log_events.write(TerminalLogEvent {
            message: format!(
                "Removed road between ({}, {}) and ({}, {})",
                edge.0.x, edge.0.y, edge.1.x, edge.1.y
            ),
        });
    } else {
        let cost: i64 = 10;
        if let Some(player) = &player
            && let Ok(mut treasury) = treasuries.get_mut(player.0)
        {
            if treasury.0 >= cost {
                treasury.0 -= cost;
                roads.0.insert(edge);
                log_events.write(TerminalLogEvent {
                    message: format!(
                        "Built road between ({}, {}) and ({}, {}) for ${}",
                        edge.0.x, edge.0.y, edge.1.x, edge.1.y, cost
                    ),
                });
            } else {
                log_events.write(TerminalLogEvent {
                    message: format!(
                        "Not enough money to build road (need ${}, have ${})",
                        cost, treasury.0
                    ),
                });
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
    log_events: &mut MessageWriter<TerminalLogEvent>,
) {
    if !are_adjacent(e.a, e.b) {
        return;
    }
    let edge = ordered_edge(e.a, e.b);

    // Check if rail already exists
    if rails.0.contains(&edge) {
        log_events.write(TerminalLogEvent {
            message: format!(
                "Rail already exists between ({}, {}) and ({}, {})",
                edge.0.x, edge.0.y, edge.1.x, edge.1.y
            ),
        });
        return;
    }

    // Check terrain buildability for both endpoints
    if let Some(player) = &player {
        // Get player nation's technologies
        let player_techs = nations.get(player.0).ok();

        // Check both tiles for terrain restrictions
        let mut can_build = true;
        let mut failure_reason: Option<String> = None;

        for tile_storage in tile_storage_query.iter() {
            // Check tile A - if we can't find it, fail the build
            match tile_storage.get(&e.a) {
                Some(tile_entity_a) => {
                    if let Ok(terrain_a) = tile_types.get(tile_entity_a)
                        && let Some(techs) = player_techs
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
                        && let Some(techs) = player_techs
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
            log_events.write(TerminalLogEvent {
                message: failure_reason
                    .unwrap_or_else(|| "Cannot build rail on this terrain".to_string()),
            });
            return;
        }
    }

    // Start rail construction (takes 3 turns)
    let cost: i64 = 50;
    if let Some(player) = &player
        && let Ok(mut treasury) = treasuries.get_mut(player.0)
    {
        if treasury.0 >= cost {
            treasury.0 -= cost;

            // Use the engineer from the message, or a dummy entity if not provided
            let engineer = e.engineer.unwrap_or(player.0);

            commands.spawn(RailConstruction {
                from: edge.0,
                to: edge.1,
                turns_remaining: 3,
                owner: player.0,
                engineer,
            });

            log_events.write(TerminalLogEvent {
                message: format!(
                    "Started rail construction from ({}, {}) to ({}, {}) for ${} (3 turns)",
                    edge.0.x, edge.0.y, edge.1.x, edge.1.y, cost
                ),
            });
        } else {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Not enough money to build rail (need ${}, have ${})",
                    cost, treasury.0
                ),
            });
        }
    }
}

fn handle_depot_placement(
    commands: &mut Commands,
    a: TilePos,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
    log_events: &mut MessageWriter<TerminalLogEvent>,
) {
    // Depot is placed on a single tile (use position 'a', ignore 'b')
    let cost: i64 = 100;
    if let Some(player) = &player
        && let Ok(mut treasury) = treasuries.get_mut(player.0)
    {
        if treasury.0 >= cost {
            treasury.0 -= cost;
            commands.spawn(Depot {
                position: a,
                owner: player.0,  // Set owner to player nation
                connected: false, // Will be computed by connectivity system
            });
            log_events.write(TerminalLogEvent {
                message: format!("Built depot at ({}, {}) for ${}", a.x, a.y, cost),
            });
        } else {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Not enough money to build depot (need ${}, have ${})",
                    cost, treasury.0
                ),
            });
        }
    }
}

fn handle_port_placement(
    commands: &mut Commands,
    a: TilePos,
    player: &Option<Res<PlayerNation>>,
    treasuries: &mut Query<&mut Treasury>,
    tile_storage_query: &Query<&TileStorage>,
    tile_types: &Query<&TerrainType>,
    log_events: &mut MessageWriter<TerminalLogEvent>,
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
        log_events.write(TerminalLogEvent {
            message: format!(
                "Cannot build port at ({}, {}): must be adjacent to water",
                port_pos.x, port_pos.y
            ),
        });
        return;
    }

    // Port is placed on a single tile
    let cost: i64 = 150;
    if let Some(player) = &player
        && let Ok(mut treasury) = treasuries.get_mut(player.0)
    {
        if treasury.0 >= cost {
            treasury.0 -= cost;
            commands.spawn(Port {
                position: a,
                owner: player.0, // Set owner to player nation
                connected: false,
                is_river: false, // TODO: detect from terrain
            });
            log_events.write(TerminalLogEvent {
                message: format!("Built port at ({}, {}) for ${}", a.x, a.y, cost),
            });
        } else {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Not enough money to build port (need ${}, have ${})",
                    cost, treasury.0
                ),
            });
        }
    }
}
