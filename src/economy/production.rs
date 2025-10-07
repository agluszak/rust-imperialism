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
        let neighbors = center_hex.all_neighbors();
        let tiles_to_check = neighbors.iter().copied().chain(std::iter::once(center_hex));

        for hex in tiles_to_check {
            if let Some(tile_pos) = hex.to_tile_pos() {
                if processed_tiles.contains(&tile_pos) {
                    continue; // Avoid double-counting
                }
                if let Some(tile_entity) = tile_storage.get(&tile_pos)
                    && let Ok(resource) = tile_resources.get(tile_entity)
                    && resource.discovered
                    && resource.get_output() > 0
                {
                    let nation_production = production.0.entry(owner).or_default();
                    let entry = nation_production.entry(resource.resource_type).or_default();
                    entry.0 += 1; // Increment improvement count
                    entry.1 += resource.get_output(); // Add production output
                    processed_tiles.insert(tile_pos);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingKind {
    TextileMill,    // 2×Cotton OR 2×Wool → 1×Cloth
}

/// What input material a building should use for production
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionChoice {
    // For TextileMill: choose between Cotton or Wool
    UseCotton,
    UseWool,
}

/// Production settings for a building (persists turn-to-turn)
#[derive(Component, Debug, Clone)]
pub struct ProductionSettings {
    /// What input material to use (e.g., Cotton vs Wool for textile mill)
    pub choice: ProductionChoice,
    /// How many units to produce this turn (capped by capacity and inputs)
    pub target_output: u32,
}

impl Default for ProductionSettings {
    fn default() -> Self {
        Self {
            choice: ProductionChoice::UseCotton,
            target_output: 0,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Building {
    pub kind: BuildingKind,
    pub capacity: u32, // Maximum output per turn
}

impl Building {
    pub fn textile_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::TextileMill,
            capacity,
        }
    }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// Automatically reduces production settings if inputs are unavailable.
/// Production rules follow 2:1 ratios (2 inputs → 1 output).
pub fn run_production(
    turn: Res<crate::turn_system::TurnSystem>,
    mut q: Query<(&mut Stockpile, &Building, &mut ProductionSettings)>,
) {
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (mut stock, building, mut settings) in q.iter_mut() {
        match building.kind {
            BuildingKind::TextileMill => {
                // 2:1 ratio: 2×Cotton OR 2×Wool → 1×Cloth

                // Determine available input based on choice
                let input_good = match settings.choice {
                    ProductionChoice::UseCotton => Good::Cotton,
                    ProductionChoice::UseWool => Good::Wool,
                };

                let available_input = stock.get(input_good);

                // Calculate how much we can produce:
                // - Limited by target_output (what user requested)
                // - Limited by capacity (building max)
                // - Limited by inputs available (need 2 inputs per 1 output)
                let max_from_inputs = available_input / 2; // 2:1 ratio
                let actual_output = settings.target_output
                    .min(building.capacity)
                    .min(max_from_inputs);

                if actual_output > 0 {
                    // Consume 2 inputs per output
                    let inputs_needed = actual_output * 2;
                    let consumed = stock.take_up_to(input_good, inputs_needed);

                    // Produce outputs (should be actual_output)
                    let produced = consumed / 2;
                    stock.add(Good::Cloth, produced);

                    // Auto-reduce target if we couldn't meet it (inputs ran out)
                    if produced < settings.target_output {
                        settings.target_output = produced;
                    }
                }
            }
        }
    }
}
