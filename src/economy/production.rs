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
    // Production buildings
    TextileMill,          // 2×Cotton OR 2×Wool → 1×Fabric
    LumberMill,           // 2×Timber → 1×Lumber OR 1×Paper
    SteelMill,            // 1×Iron + 1×Coal → 1×Steel
    FoodProcessingCenter, // 2×Grain + 1×Fruit + 1×(Livestock|Fish) → 2×CannedFood

    // Worker-related buildings (no production capacity)
    Capitol,     // Recruit untrained workers
    TradeSchool, // Train workers
    PowerPlant,  // Convert fuel to labor
}

/// What input material a building should use for production
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionChoice {
    // For TextileMill: choose between Cotton or Wool
    UseCotton,
    UseWool,

    // For LumberMill: choose between Lumber or Paper
    MakeLumber,
    MakePaper,

    // For FoodProcessingCenter: choose between Livestock or Fish
    UseLivestock,
    UseFish,
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

    pub fn lumber_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::LumberMill,
            capacity,
        }
    }

    pub fn steel_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::SteelMill,
            capacity,
        }
    }

    pub fn food_processing_center(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::FoodProcessingCenter,
            capacity,
        }
    }

    pub fn capitol() -> Self {
        Self {
            kind: BuildingKind::Capitol,
            capacity: 0, // Not a production building
        }
    }

    pub fn trade_school() -> Self {
        Self {
            kind: BuildingKind::TradeSchool,
            capacity: 0, // Not a production building
        }
    }

    pub fn power_plant(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::PowerPlant,
            capacity, // Fuel → labor conversion capacity
        }
    }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// Automatically reduces production settings if inputs are unavailable.
/// Production rules follow 2:1 ratios (2 inputs → 1 output).
/// Production now requires labor points from workers.
pub fn run_production(
    turn: Res<crate::turn_system::TurnSystem>,
    mut q: Query<(
        Option<&super::workforce::Workforce>,
        &mut Stockpile,
        &Building,
        &mut ProductionSettings,
    )>,
) {
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (workforce_opt, mut stock, building, mut settings) in q.iter_mut() {
        // Calculate available labor (0 if no workforce)
        let available_labor = workforce_opt.map(|w| w.available_labor()).unwrap_or(0);

        // Each unit of production requires 1 labor point
        // This acts as another constraint on production alongside capacity and inputs
        let max_from_labor = available_labor;
        match building.kind {
            BuildingKind::TextileMill => {
                // 2:1 ratio: 2×Cotton OR 2×Wool → 1×Fabric

                let input_good = match settings.choice {
                    ProductionChoice::UseCotton => Good::Cotton,
                    ProductionChoice::UseWool => Good::Wool,
                    _ => continue, // Invalid choice for this building
                };

                let available_input = stock.get(input_good);
                let max_from_inputs = available_input / 2; // 2:1 ratio
                let actual_output = settings
                    .target_output
                    .min(building.capacity)
                    .min(max_from_inputs)
                    .min(max_from_labor); // Also limited by labor

                if actual_output > 0 {
                    let inputs_needed = actual_output * 2;
                    let consumed = stock.take_up_to(input_good, inputs_needed);
                    let produced = consumed / 2;
                    stock.add(Good::Fabric, produced);

                    if produced < settings.target_output {
                        settings.target_output = produced;
                    }
                }
            }

            BuildingKind::LumberMill => {
                // 2:1 ratio: 2×Timber → 1×Lumber OR 1×Paper

                let output_good = match settings.choice {
                    ProductionChoice::MakeLumber => Good::Lumber,
                    ProductionChoice::MakePaper => Good::Paper,
                    _ => continue,
                };

                let available_timber = stock.get(Good::Timber);
                let max_from_inputs = available_timber / 2;
                let actual_output = settings
                    .target_output
                    .min(building.capacity)
                    .min(max_from_inputs)
                    .min(max_from_labor);

                if actual_output > 0 {
                    let inputs_needed = actual_output * 2;
                    let consumed = stock.take_up_to(Good::Timber, inputs_needed);
                    let produced = consumed / 2;
                    stock.add(output_good, produced);

                    if produced < settings.target_output {
                        settings.target_output = produced;
                    }
                }
            }

            BuildingKind::SteelMill => {
                // 1:1 ratio: 1×Iron + 1×Coal → 1×Steel

                let available_iron = stock.get(Good::Iron);
                let available_coal = stock.get(Good::Coal);
                let max_from_inputs = available_iron.min(available_coal);
                let actual_output = settings
                    .target_output
                    .min(building.capacity)
                    .min(max_from_inputs)
                    .min(max_from_labor);

                if actual_output > 0 {
                    stock.take_up_to(Good::Iron, actual_output);
                    stock.take_up_to(Good::Coal, actual_output);
                    stock.add(Good::Steel, actual_output);

                    if actual_output < settings.target_output {
                        settings.target_output = actual_output;
                    }
                }
            }

            BuildingKind::FoodProcessingCenter => {
                // Special ratio: 2×Grain + 1×Fruit + 1×(Livestock|Fish) → 2×CannedFood

                let meat_good = match settings.choice {
                    ProductionChoice::UseLivestock => Good::Livestock,
                    ProductionChoice::UseFish => Good::Fish,
                    _ => continue,
                };

                let available_grain = stock.get(Good::Grain);
                let available_fruit = stock.get(Good::Fruit);
                let available_meat = stock.get(meat_good);

                // Each batch needs: 2 grain, 1 fruit, 1 meat → produces 2 canned food
                let max_batches_from_grain = available_grain / 2;
                let max_batches_from_fruit = available_fruit;
                let max_batches_from_meat = available_meat;
                let max_batches = max_batches_from_grain
                    .min(max_batches_from_fruit)
                    .min(max_batches_from_meat);

                // target_output is in canned food units, so divide by 2 to get batches
                let target_batches = settings.target_output.div_ceil(2); // Round up

                // Labor constraint: 2 canned food needs 2 labor (1 labor per output)
                let max_batches_from_labor = max_from_labor / 2;

                let actual_batches = target_batches
                    .min(building.capacity / 2)
                    .min(max_batches)
                    .min(max_batches_from_labor);

                if actual_batches > 0 {
                    stock.take_up_to(Good::Grain, actual_batches * 2);
                    stock.take_up_to(Good::Fruit, actual_batches);
                    stock.take_up_to(meat_good, actual_batches);
                    let produced = actual_batches * 2;
                    stock.add(Good::CannedFood, produced);

                    if produced < settings.target_output {
                        settings.target_output = produced;
                    }
                }
            }

            // Worker-related buildings don't run in this system
            BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => {}
        }
    }
}
