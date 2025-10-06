use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet, VecDeque};

use super::{nation::PlayerNation, treasury::Treasury};
use crate::tile_pos::TilePosExt;
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

fn ordered_edge(a: TilePos, b: TilePos) -> (TilePos, TilePos) {
    if (a.x, a.y) <= (b.x, b.y) {
        (a, b)
    } else {
        (b, a)
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

/// Apply improvement placements (roads, rails, depots, ports) and charge the player treasury
pub fn apply_improvements(
    mut commands: Commands,
    mut ev: MessageReader<PlaceImprovement>,
    mut roads: ResMut<Roads>,
    mut rails: ResMut<Rails>,
    player: Option<Res<PlayerNation>>,
    mut treasuries: Query<&mut Treasury>,
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
                // Toggle behavior for rails
                if rails.0.contains(&edge) {
                    rails.0.remove(&edge);
                    log_events.write(TerminalLogEvent {
                        message: format!(
                            "Removed rail between ({}, {}) and ({}, {})",
                            edge.0.x, edge.0.y, edge.1.x, edge.1.y
                        ),
                    });
                } else {
                    let cost: i64 = 50; // Rails cost more than roads
                    if let Some(player) = &player
                        && let Ok(mut treasury) = treasuries.get_mut(player.0)
                    {
                        if treasury.0 >= cost {
                            treasury.0 -= cost;
                            rails.0.insert(edge);
                            log_events.write(TerminalLogEvent {
                                message: format!(
                                    "Built rail between ({}, {}) and ({}, {}) for ${}",
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
