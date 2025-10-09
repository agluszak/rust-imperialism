use bevy::prelude::*;
use std::collections::HashMap;

use super::{
    goods::Good,
    production::{BuildingKind, ProductionChoice},
    workforce::WorkerSkill,
};

/// Per-nation component tracking all resource allocations for the current turn.
/// Allocations represent player intent and are adjustable during PlayerTurn.
/// At turn end, allocations are converted to reservations and queued orders.
#[derive(Component, Debug, Clone, Default)]
pub struct ResourceAllocations {
    /// Worker recruitment allocation (Capitol building)
    pub recruitment: RecruitmentAllocation,
    /// Worker training allocations (Trade School)
    pub training: Vec<TrainingAllocation>,
    /// Production allocations per building
    pub production: HashMap<Entity, ProductionAllocation>,
}

/// Allocation for recruiting untrained workers at the Capitol
#[derive(Debug, Clone, Default)]
pub struct RecruitmentAllocation {
    /// How many workers the player wants to recruit
    pub requested: u32,
    /// How many can actually be allocated given constraints
    /// (limited by capacity cap, resources, or both)
    pub allocated: u32,
}

impl RecruitmentAllocation {
    /// Per-unit input requirements for recruitment
    /// 1 CannedFood + 1 Clothing + 1 Furniture → 1 Untrained Worker
    pub fn inputs_per_unit() -> Vec<(Good, u32)> {
        vec![
            (Good::CannedFood, 1),
            (Good::Clothing, 1),
            (Good::Furniture, 1),
        ]
    }

    /// Calculate total inputs needed for allocated amount
    pub fn total_inputs_needed(&self) -> Vec<(Good, u32)> {
        Self::inputs_per_unit()
            .into_iter()
            .map(|(good, qty)| (good, qty * self.allocated))
            .collect()
    }
}

/// Allocation for training workers at the Trade School
#[derive(Debug, Clone)]
pub struct TrainingAllocation {
    /// Which skill level to train from
    pub from_skill: WorkerSkill,
    /// How many workers the player wants to train
    pub requested: u32,
    /// How many can actually be allocated
    pub allocated: u32,
}

impl TrainingAllocation {
    /// Per-unit input requirements for training
    /// 1 Paper + $100 → train 1 worker to next level
    pub fn inputs_per_unit() -> Vec<(Good, u32)> {
        vec![(Good::Paper, 1)]
    }

    pub fn cash_per_unit() -> i64 {
        100
    }

    /// Calculate total inputs needed for allocated amount
    pub fn total_inputs_needed(&self) -> Vec<(Good, u32)> {
        Self::inputs_per_unit()
            .into_iter()
            .map(|(good, qty)| (good, qty * self.allocated))
            .collect()
    }

    pub fn total_cash_needed(&self) -> i64 {
        Self::cash_per_unit() * self.allocated as i64
    }
}

/// Allocation for production at a specific building
#[derive(Debug, Clone)]
pub struct ProductionAllocation {
    /// What type of building this is for
    pub building_kind: BuildingKind,
    /// Production choice (e.g., UseCotton vs UseWool for textile mill)
    pub choice: ProductionChoice,
    /// Building's maximum capacity
    pub capacity: u32,
    /// Per-output allocations (Good -> requested/allocated amounts)
    /// For buildings producing single output, will have one entry
    /// For lumber mill/metalworks, will have two entries (e.g., Paper + Lumber)
    pub outputs: HashMap<Good, OutputAllocation>,
}

/// Tracks allocation for a specific output good from a building
#[derive(Debug, Clone, Default)]
pub struct OutputAllocation {
    /// Target output requested by player
    pub requested: u32,
    /// Actual allocated output (limited by capacity, inputs, labor)
    pub allocated: u32,
}

impl ProductionAllocation {
    /// Calculate input requirements for ALL allocated outputs
    /// Returns inputs needed as Vec<(Good, quantity)>
    pub fn inputs_needed(&self) -> Vec<(Good, u32)> {
        let mut total_inputs: HashMap<Good, u32> = HashMap::new();

        for (output_good, output_alloc) in &self.outputs {
            let allocated = output_alloc.allocated;
            if allocated == 0 {
                continue;
            }

            match self.building_kind {
                BuildingKind::TextileMill => {
                    // 2×Fiber (Cotton + Wool) → 1×Fabric
                    // Special case: handled in allocation_systems
                }

                BuildingKind::LumberMill => {
                    // 2×Timber → 1×(Lumber|Paper)
                    *total_inputs.entry(Good::Timber).or_insert(0) += allocated * 2;
                }

                BuildingKind::SteelMill => {
                    // 1×Iron + 1×Coal → 1×Steel
                    *total_inputs.entry(Good::Iron).or_insert(0) += allocated;
                    *total_inputs.entry(Good::Coal).or_insert(0) += allocated;
                }

                BuildingKind::FoodProcessingCenter => {
                    // 2×Grain + 1×Fruit + 1×(Livestock|Fish) → 2×CannedFood
                    let meat_good = match self.choice {
                        ProductionChoice::UseLivestock => Good::Livestock,
                        ProductionChoice::UseFish => Good::Fish,
                        _ => continue,
                    };

                    let batches = allocated.div_ceil(2);
                    *total_inputs.entry(Good::Grain).or_insert(0) += batches * 2;
                    *total_inputs.entry(Good::Fruit).or_insert(0) += batches;
                    *total_inputs.entry(meat_good).or_insert(0) += batches;
                }

                // Non-production buildings
                BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => {}
            }
        }

        total_inputs.into_iter().collect()
    }

    /// Get the output good(s) produced by this allocation
    pub fn outputs_produced(&self) -> Vec<(Good, u32)> {
        self.outputs
            .iter()
            .map(|(good, alloc)| (*good, alloc.allocated))
            .filter(|(_, qty)| *qty > 0)
            .collect()
    }

    /// Get total allocated output across all outputs
    pub fn total_allocated(&self) -> u32 {
        self.outputs.values().map(|a| a.allocated).sum()
    }
}

// ============================================================================
// Messages (Input Layer)
// ============================================================================

/// Player adjusts recruitment allocation (Capitol building)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustRecruitment {
    pub nation: Entity,
    pub requested: u32,
}

/// Player adjusts training allocation (Trade School)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustTraining {
    pub nation: Entity,
    pub from_skill: WorkerSkill,
    pub requested: u32,
}

/// Player adjusts production allocation (mills/factories)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustProduction {
    pub nation: Entity,
    pub building: Entity,
    pub output_good: Good, // Which output to adjust (Paper, Lumber, etc.)
    pub choice: Option<ProductionChoice>, // None = keep current choice
    pub target_output: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruitment_allocation_inputs() {
        let alloc = RecruitmentAllocation {
            requested: 10,
            allocated: 5,
        };

        let inputs = alloc.total_inputs_needed();
        assert_eq!(inputs.len(), 3);
        assert!(inputs.contains(&(Good::CannedFood, 5)));
        assert!(inputs.contains(&(Good::Clothing, 5)));
        assert!(inputs.contains(&(Good::Furniture, 5)));
    }

    #[test]
    fn training_allocation_costs() {
        let alloc = TrainingAllocation {
            from_skill: WorkerSkill::Untrained,
            requested: 8,
            allocated: 3,
        };

        let inputs = alloc.total_inputs_needed();
        assert_eq!(inputs, vec![(Good::Paper, 3)]);
        assert_eq!(alloc.total_cash_needed(), 300);
    }

    #[test]
    fn textile_mill_allocation_inputs() {
        let mut outputs = HashMap::new();
        outputs.insert(
            Good::Fabric,
            OutputAllocation {
                requested: 10,
                allocated: 4, // Limited by something (inputs/labor)
            },
        );

        let alloc = ProductionAllocation {
            building_kind: BuildingKind::TextileMill,
            choice: ProductionChoice::UseCotton,
            capacity: 8,
            outputs,
        };

        let inputs = alloc.inputs_needed();
        // TextileMill returns empty vec (handled specially in allocation_systems)
        assert_eq!(inputs, vec![]);

        let produced = alloc.outputs_produced();
        assert_eq!(produced, vec![(Good::Fabric, 4)]);
    }

    #[test]
    fn steel_mill_allocation_inputs() {
        let mut outputs = HashMap::new();
        outputs.insert(
            Good::Steel,
            OutputAllocation {
                requested: 5,
                allocated: 5,
            },
        );

        let alloc = ProductionAllocation {
            building_kind: BuildingKind::SteelMill,
            choice: ProductionChoice::UseCotton, // Ignored for steel mill
            capacity: 5,
            outputs,
        };

        let inputs = alloc.inputs_needed();
        assert_eq!(inputs.len(), 2);
        assert!(inputs.contains(&(Good::Iron, 5)));
        assert!(inputs.contains(&(Good::Coal, 5)));
    }

    #[test]
    fn food_processing_allocation_inputs() {
        let mut outputs = HashMap::new();
        outputs.insert(
            Good::CannedFood,
            OutputAllocation {
                requested: 4,
                allocated: 4,
            },
        );

        let alloc = ProductionAllocation {
            building_kind: BuildingKind::FoodProcessingCenter,
            choice: ProductionChoice::UseLivestock,
            capacity: 4,
            outputs,
        };

        let inputs = alloc.inputs_needed();
        // 4 output → 2 batches → 4 grain, 2 fruit, 2 livestock
        assert_eq!(inputs.len(), 3);
        assert!(inputs.contains(&(Good::Grain, 4)));
        assert!(inputs.contains(&(Good::Fruit, 2)));
        assert!(inputs.contains(&(Good::Livestock, 2)));
    }
}
