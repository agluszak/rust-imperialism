use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet, VecDeque};

use super::{nation::PlayerNation, treasury::Treasury};
use crate::tile_pos::{HexExt, TilePosExt};
use crate::ui::logging::TerminalLogEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementKind {
    Road,  // Early-game low-capacity transport
    Rail,  // High-capacity transport network
    Depot, // Gathers resources from tile + 8 neighbors
    Port,  // Coastal/river gathering point
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlaceImprovement {
    pub a: TilePos,
    pub b: TilePos,
    pub kind: ImprovementKind,
    pub engineer: Option<Entity>, // Engineer entity building this (for tracking construction)
}

/// Marker component for depots that gather resources
#[derive(Component, Debug)]
pub struct Depot {
    pub position: TilePos,
    pub owner: Entity,   // Nation entity that owns this depot
    pub connected: bool, // Whether this depot has a rail path to owner's capital
}

/// Marker component for ports (coastal or river)
#[derive(Component, Debug)]
pub struct Port {
    pub position: TilePos,
    pub owner: Entity, // Nation entity that owns this port
    pub connected: bool,
    pub is_river: bool,
}

/// Roads are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug)]
pub struct Roads(pub HashSet<(TilePos, TilePos)>);

/// Rails are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug)]
pub struct Rails(pub HashSet<(TilePos, TilePos)>);

/// Component tracking rail construction in progress (takes 3 turns to complete)
#[derive(Component, Debug)]
pub struct RailConstruction {
    pub from: TilePos,
    pub to: TilePos,
    pub turns_remaining: u32,
    pub owner: Entity,    // Nation that started construction
    pub engineer: Entity, // Engineer entity that is building this
}

fn ordered_edge(a: TilePos, b: TilePos) -> (TilePos, TilePos) {
    if (a.x, a.y) <= (b.x, b.y) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Advance rail construction progress each turn
pub fn advance_rail_construction(
    mut commands: Commands,
    mut constructions: Query<(Entity, &mut RailConstruction)>,
    mut rails: ResMut<Rails>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut construction) in constructions.iter_mut() {
        construction.turns_remaining -= 1;

        if construction.turns_remaining == 0 {
            // Construction complete!
            let edge = ordered_edge(construction.from, construction.to);
            rails.0.insert(edge);

            log_events.write(TerminalLogEvent {
                message: format!(
                    "Rail construction complete: ({}, {}) to ({}, {})",
                    edge.0.x, edge.0.y, edge.1.x, edge.1.y
                ),
            });

            // Remove construction entity
            commands.entity(entity).despawn();
        } else {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Rail construction: ({}, {}) to ({}, {}) - {} turns remaining",
                    construction.from.x,
                    construction.from.y,
                    construction.to.x,
                    construction.to.y,
                    construction.turns_remaining
                ),
            });
        }
    }
}

/// Build adjacency list for BFS from rail edges
fn build_rail_graph(rails: &Rails) -> HashMap<TilePos, Vec<TilePos>> {
    let mut graph: HashMap<TilePos, Vec<TilePos>> = HashMap::new();
    for &(a, b) in rails.0.iter() {
        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }
    graph
}

fn are_adjacent(a: TilePos, b: TilePos) -> bool {
    let ha = a.to_hex();
    let hb = b.to_hex();
    ha.distance_to(hb) == 1
}

use super::technology::{Technologies, Technology};
use crate::tiles::TerrainType;
use bevy_ecs_tilemap::prelude::TileStorage;

/// Check if terrain is buildable for rails given technologies
fn can_build_rail_on_terrain(
    terrain: &TerrainType,
    technologies: &Technologies,
) -> (bool, Option<&'static str>) {
    match terrain {
        TerrainType::Water => {
            // Cannot build rails on water
            (false, Some("Cannot build rails on water"))
        }
        TerrainType::Mountain => {
            if technologies.has(Technology::MountainEngineering) {
                (true, None)
            } else {
                (false, Some("Mountain Engineering technology required"))
            }
        }
        TerrainType::Hills => {
            if technologies.has(Technology::HillGrading) {
                (true, None)
            } else {
                (false, Some("Hill Grading technology required"))
            }
        }
        TerrainType::Swamp => {
            if technologies.has(Technology::SwampDrainage) {
                (true, None)
            } else {
                (false, Some("Swamp Drainage technology required"))
            }
        }
        _ => (true, None), // All other terrains are buildable by default
    }
}

/// Apply improvement placements (roads, rails, depots, ports) and charge the player treasury
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
                if !are_adjacent(e.a, e.b) {
                    continue;
                }
                let edge = ordered_edge(e.a, e.b);
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
            ImprovementKind::Rail => {
                if !are_adjacent(e.a, e.b) {
                    continue;
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
                    continue;
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
                                    let (buildable, reason) =
                                        can_build_rail_on_terrain(terrain_a, techs);
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
                                failure_reason =
                                    Some(format!("Tile ({}, {}) not found", e.a.x, e.a.y));
                                break;
                            }
                        }

                        // Check tile B - if we can't find it, fail the build
                        match tile_storage.get(&e.b) {
                            Some(tile_entity_b) => {
                                if let Ok(terrain_b) = tile_types.get(tile_entity_b)
                                    && let Some(techs) = player_techs
                                {
                                    let (buildable, reason) =
                                        can_build_rail_on_terrain(terrain_b, techs);
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
                                failure_reason =
                                    Some(format!("Tile ({}, {}) not found", e.b.x, e.b.y));
                                break;
                            }
                        }
                    }

                    if !can_build {
                        log_events.write(TerminalLogEvent {
                            message: failure_reason
                                .unwrap_or_else(|| "Cannot build rail on this terrain".to_string()),
                        });
                        continue;
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
            ImprovementKind::Depot => {
                // Depot is placed on a single tile (use position 'a', ignore 'b')
                let cost: i64 = 100;
                if let Some(player) = &player
                    && let Ok(mut treasury) = treasuries.get_mut(player.0)
                {
                    if treasury.0 >= cost {
                        treasury.0 -= cost;
                        commands.spawn(Depot {
                            position: e.a,
                            owner: player.0,  // Set owner to player nation
                            connected: false, // Will be computed by connectivity system
                        });
                        log_events.write(TerminalLogEvent {
                            message: format!("Built depot at ({}, {}) for ${}", e.a.x, e.a.y, cost),
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
            ImprovementKind::Port => {
                // Port must be adjacent to water
                let port_pos = e.a;
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
                    continue;
                }

                // Port is placed on a single tile
                let cost: i64 = 150;
                if let Some(player) = &player
                    && let Ok(mut treasury) = treasuries.get_mut(player.0)
                {
                    if treasury.0 >= cost {
                        treasury.0 -= cost;
                        commands.spawn(Port {
                            position: e.a,
                            owner: player.0, // Set owner to player nation
                            connected: false,
                            is_river: false, // TODO: detect from terrain
                        });
                        log_events.write(TerminalLogEvent {
                            message: format!("Built port at ({}, {}) for ${}", e.a.x, e.a.y, cost),
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
        }
    }
}

/// Compute rail network connectivity for all nations
/// Uses BFS from each nation's capital to mark depots/ports as connected
pub fn compute_rail_connectivity(
    rails: Res<Rails>,
    nations: Query<(Entity, &super::nation::Capital)>,
    mut depots: Query<&mut Depot>,
    mut ports: Query<&mut Port>,
) {
    // Build the rail graph once
    let graph = build_rail_graph(&rails);

    // For each nation, run BFS from their capital
    for (nation_entity, capital) in nations.iter() {
        let capital_pos = capital.0;

        // BFS to find all reachable tiles from this capital
        let mut reachable: HashSet<TilePos> = HashSet::new();
        let mut queue: VecDeque<TilePos> = VecDeque::new();

        queue.push_back(capital_pos);
        reachable.insert(capital_pos);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if !reachable.contains(&neighbor) {
                        reachable.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Update depots owned by this nation
        for mut depot in depots.iter_mut() {
            if depot.owner == nation_entity {
                depot.connected = reachable.contains(&depot.position);
            }
        }

        // Update ports owned by this nation
        for mut port in ports.iter_mut() {
            if port.owner == nation_entity {
                port.connected = reachable.contains(&port.position);
            }
        }
    }
}
