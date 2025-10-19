use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::iter;

use crate::{
    civilians::types::ProspectingKnowledge,
    economy::transport::{Depot, Port},
    map::tile_pos::{HexExt, TilePosExt},
    resources::{ResourceType, TileResource},
};
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use super::workforce::Workforce;
use super::{goods::Good, stockpile::Stockpile};
use crate::turn_system::{TurnPhase, TurnSystem};

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
    prospecting_knowledge: Res<ProspectingKnowledge>,
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
        let tiles_to_check = neighbors.iter().copied().chain(iter::once(center_hex));

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
                    if resource.requires_prospecting()
                        && !prospecting_knowledge.is_discovered_by(tile_entity, owner)
                    {
                        continue;
                    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Collection of all buildings for a nation
#[derive(Component, Debug, Clone, Default)]
pub struct Buildings {
    pub buildings: HashMap<BuildingKind, Building>,
}

impl Buildings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_all_initial() -> Self {
        let mut buildings = HashMap::new();
        buildings.insert(BuildingKind::TextileMill, Building::textile_mill(8));
        buildings.insert(BuildingKind::LumberMill, Building::lumber_mill(4));
        buildings.insert(BuildingKind::SteelMill, Building::steel_mill(4));
        buildings.insert(
            BuildingKind::FoodProcessingCenter,
            Building::food_processing_center(4),
        );
        Self { buildings }
    }

    pub fn get(&self, kind: BuildingKind) -> Option<Building> {
        self.buildings.get(&kind).copied()
    }

    pub fn insert(&mut self, building: Building) {
        self.buildings.insert(building.kind, building);
    }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// Consumes reserved resources and produces outputs.
/// Production rules follow 2:1 ratios (2 inputs → 1 output).
/// Production now requires labor points from workers.
pub fn run_production(
    turn: Res<TurnSystem>,
    mut q: Query<(
        Option<&Workforce>,
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

                // Apply labor constraint
                let actual_output = settings.target_output.min(max_from_labor);

                if actual_output > 0 {
                    let inputs_needed = actual_output * 2;
                    // Consume reserved inputs (should already be reserved by UI)
                    let consumed = stock.consume_reserved(input_good, inputs_needed);
                    let produced = consumed / 2;
                    stock.add(Good::Fabric, produced);

                    if produced < actual_output {
                        // This shouldn't happen if reservations work correctly
                        info!(
                            "TextileMill: expected {} but only consumed {} inputs",
                            inputs_needed, consumed
                        );
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

                // Apply labor constraint
                let actual_output = settings.target_output.min(max_from_labor);

                if actual_output > 0 {
                    let inputs_needed = actual_output * 2;
                    let consumed = stock.consume_reserved(Good::Timber, inputs_needed);
                    let produced = consumed / 2;
                    stock.add(output_good, produced);

                    if produced < actual_output {
                        info!(
                            "LumberMill: expected {} but only consumed {} inputs",
                            inputs_needed, consumed
                        );
                        settings.target_output = produced;
                    }
                }
            }

            BuildingKind::SteelMill => {
                // 1:1 ratio: 1×Iron + 1×Coal → 1×Steel

                // Apply labor constraint
                let actual_output = settings.target_output.min(max_from_labor);

                if actual_output > 0 {
                    let iron_consumed = stock.consume_reserved(Good::Iron, actual_output);
                    let coal_consumed = stock.consume_reserved(Good::Coal, actual_output);
                    let produced = iron_consumed.min(coal_consumed);
                    stock.add(Good::Steel, produced);

                    if produced < actual_output {
                        info!(
                            "SteelMill: expected {} but only consumed {} iron and {} coal",
                            actual_output, iron_consumed, coal_consumed
                        );
                        settings.target_output = produced;
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

                // target_output is in canned food units (comes in pairs)
                // Apply labor constraint (1 labor per output unit)
                let actual_output = settings.target_output.min(max_from_labor);
                let target_batches = actual_output.div_ceil(2); // Round up to batches

                if target_batches > 0 {
                    // Each batch: 2 grain, 1 fruit, 1 meat → 2 canned food
                    let grain_consumed = stock.consume_reserved(Good::Grain, target_batches * 2);
                    let fruit_consumed = stock.consume_reserved(Good::Fruit, target_batches);
                    let meat_consumed = stock.consume_reserved(meat_good, target_batches);

                    // Calculate actual batches we can produce from what was consumed
                    let actual_batches =
                        (grain_consumed / 2).min(fruit_consumed).min(meat_consumed);

                    let produced = actual_batches * 2;
                    stock.add(Good::CannedFood, produced);

                    if produced < actual_output {
                        info!(
                            "FoodProcessingCenter: expected {} but only consumed {} grain, {} fruit, {} meat",
                            target_batches * 2,
                            grain_consumed,
                            fruit_consumed,
                            meat_consumed
                        );
                        settings.target_output = produced;
                    }
                }
            }

            // Worker-related buildings don't run in this system
            BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => {}
        }
    }
}
