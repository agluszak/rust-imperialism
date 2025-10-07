use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    economy::transport::{Depot, Port},
    resources::{ResourceType, TileResource},
    tile_pos::{HexExt, TilePosExt},
};
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use super::{goods::Good, stockpile::Stockpile};
use crate::turn_system::TurnPhase;

/// Resource that stores the total connected production output for each nation.
/// `(u32, u32)` is `(number_of_improvements, total_output)`
#[derive(Resource, Default, Debug)]
pub struct ConnectedProduction(pub HashMap<Entity, HashMap<ResourceType, (u32, u32)>>);

/// Calculates the total production from all resource tiles connected to the rail network.
/// This system runs after `compute_rail_connectivity`.
pub fn calculate_connected_production(
    mut production: ResMut<ConnectedProduction>,
    connected_depots: Query<&Depot>,
    connected_ports: Query<&Port>,
    tile_storage: Query<&TileStorage>,
    tile_resources: Query<&TileResource>,
) {
    // Clear previous turn's data
    production.0.clear();

    let mut processed_tiles: HashSet<TilePos> = HashSet::new();

    let Ok(tile_storage) = tile_storage.single() else {
        return;
    };

    let mut process_improvement = |owner: Entity, position: TilePos| {
        let center_hex = position.to_hex();
        let tiles_to_check =
            center_hex.all_neighbors().iter().copied().chain(std::iter::once(center_hex));

        for hex in tiles_to_check {
            if let Some(tile_pos) = hex.to_tile_pos() {
                if processed_tiles.contains(&tile_pos) {
                    continue; // Avoid double-counting
                }
                if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                    if let Ok(resource) = tile_resources.get(tile_entity) {
                        if resource.discovered && resource.get_output() > 0 {
                            let nation_production =
                                production.0.entry(owner).or_default();
                            let entry = nation_production
                                .entry(resource.resource_type)
                                .or_default();
                            entry.0 += 1; // Increment improvement count
                            entry.1 += resource.get_output(); // Add production output
                            processed_tiles.insert(tile_pos);
                        }
                    }
                }
            }
        }
    };

    // Process connected depots
    for depot in connected_depots.iter().filter(|d| d.connected) {
        process_improvement(depot.owner, depot.position);
    }

    // Process connected ports
    for port in connected_ports.iter().filter(|p| p.connected) {
        process_improvement(port.owner, port.position);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BuildingKind {
    TextileMill,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Building {
    pub kind: BuildingKind,
    pub workers: u8,
}

impl Building {
    pub fn textile_mill(workers: u8) -> Self {
        Self {
            kind: BuildingKind::TextileMill,
            workers,
        }
    }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// For MVP, we treat buildings attached directly to nation entities and use that nation's Stockpile.
pub fn run_production(
    turn: Res<crate::turn_system::TurnSystem>,
    mut q: Query<(&mut Stockpile, &Building)>,
) {
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (mut stock, building) in q.iter_mut() {
        match building.kind {
            BuildingKind::TextileMill => {
                let workers = building.workers as u32;
                if workers == 0 {
                    continue;
                }
                let can = stock
                    .get(Good::Wool)
                    .min(stock.get(Good::Cotton))
                    .min(workers);
                if can > 0 {
                    // consume inputs
                    let _ = stock.take_up_to(Good::Wool, can);
                    let _ = stock.take_up_to(Good::Cotton, can);
                    // produce outputs
                    stock.add(Good::Cloth, can);
                }
            }
        }
    }
}
