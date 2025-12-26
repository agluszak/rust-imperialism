use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::economy::transport::messages::RecomputeConnectivity;
use crate::economy::transport::types::{Depot, Port, Rails};

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
/// Only runs when RecomputeConnectivity messages are present (topology changes)
/// Optimized to avoid O(n*m) nested iteration over nations and depots/ports
pub fn compute_rail_connectivity(
    mut events: MessageReader<RecomputeConnectivity>,
    rails: Res<Rails>,
    nations: Query<(Entity, &crate::economy::nation::Capital)>,
    mut depots: Query<&mut Depot>,
    mut ports: Query<&mut Port>,
) {
    // Only recompute when topology changed
    if events.is_empty() {
        return;
    }
    events.clear();

    // Build the rail graph once
    let graph = build_rail_graph(&rails);

    // Build a HashMap of nation reachability sets to avoid nested iteration
    let mut nation_reachable: HashMap<Entity, HashSet<TilePos>> = HashMap::new();

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

        nation_reachable.insert(nation_entity, reachable);
    }

    // Update all depots in a single pass using cached reachability sets
    // This eliminates O(n*m) nested iteration
    for mut depot in depots.iter_mut() {
        depot.connected = nation_reachable
            .get(&depot.owner)
            .is_some_and(|reachable: &HashSet<TilePos>| reachable.contains(&depot.position));
    }

    // Update all ports in a single pass using cached reachability sets
    for mut port in ports.iter_mut() {
        port.connected = nation_reachable
            .get(&port.owner)
            .is_some_and(|reachable: &HashSet<TilePos>| reachable.contains(&port.position));
    }
}

/// Observer: trigger connectivity recomputation when Depot is added
pub fn on_depot_added(_trigger: On<Add, Depot>, mut writer: MessageWriter<RecomputeConnectivity>) {
    writer.write(RecomputeConnectivity);
}

/// Observer: trigger connectivity recomputation when Depot is removed
pub fn on_depot_removed(
    _trigger: On<Remove, Depot>,
    mut writer: MessageWriter<RecomputeConnectivity>,
) {
    writer.write(RecomputeConnectivity);
}

/// Observer: trigger connectivity recomputation when Port is added
pub fn on_port_added(_trigger: On<Add, Port>, mut writer: MessageWriter<RecomputeConnectivity>) {
    writer.write(RecomputeConnectivity);
}

/// Observer: trigger connectivity recomputation when Port is removed
pub fn on_port_removed(
    _trigger: On<Remove, Port>,
    mut writer: MessageWriter<RecomputeConnectivity>,
) {
    writer.write(RecomputeConnectivity);
}
