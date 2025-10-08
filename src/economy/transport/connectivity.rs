use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet, VecDeque};

use super::types::{Depot, Port, Rails};

/// Build adjacency list for BFS from rail edges
pub fn build_rail_graph(rails: &Rails) -> HashMap<TilePos, Vec<TilePos>> {
    let mut graph: HashMap<TilePos, Vec<TilePos>> = HashMap::new();
    for &(a, b) in rails.0.iter() {
        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }
    graph
}

/// Compute rail network connectivity for all nations (Logic Layer)
/// Uses BFS from each nation's capital to mark depots/ports as connected
pub fn compute_rail_connectivity(
    rails: Res<Rails>,
    nations: Query<(Entity, &super::super::nation::Capital)>,
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
