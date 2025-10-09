use bevy::prelude::*;

use super::{
    allocation::*,
    production::{Building, ProductionSettings},
    stockpile::Stockpile,
    treasury::Treasury,
    workforce::{RecruitmentCapacity, systems::calculate_recruitment_cap, types::Workforce},
};
use crate::{province::Province, turn_system::TurnSystem};

/// System to apply recruitment allocation adjustments (Input → Logic)
/// Reads AdjustRecruitment messages and updates ResourceAllocations
/// HARD CAP: requested value is immediately capped by all constraints
pub fn apply_recruitment_adjustments(
    mut messages: MessageReader<AdjustRecruitment>,
    mut nations: Query<(&mut ResourceAllocations, &Stockpile)>,
    provinces: Query<&Province>,
    recruitment_capacity: Query<&RecruitmentCapacity>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, stockpile)) = nations.get_mut(msg.nation) else {
            warn!("Cannot adjust recruitment: nation not found");
            continue;
        };

        // Calculate capacity cap (provinces / 4 or / 3)
        let province_count = provinces
            .iter()
            .filter(|p| p.owner == Some(msg.nation))
            .count() as u32;

        let capacity_upgraded = recruitment_capacity
            .get(msg.nation)
            .map(|c| c.upgraded)
            .unwrap_or(false);

        let capacity_cap = calculate_recruitment_cap(province_count, capacity_upgraded);

        // Calculate resource cap (min of available inputs)
        let mut resource_cap = u32::MAX;
        for (good, qty_per) in RecruitmentAllocation::inputs_per_unit() {
            let available = stockpile.get_available(good);
            let max_from_this_good = available / qty_per;
            resource_cap = resource_cap.min(max_from_this_good);
        }

        // Hard cap: apply ALL constraints to requested value
        let max_possible = capacity_cap.min(resource_cap);
        let new_value = msg.requested.min(max_possible);

        // Ignore if trying to go below 0 or above max
        let current = allocations.recruitment.requested;
        if new_value == current {
            debug!("Recruitment allocation unchanged: {}", current);
            continue;
        }

        // Update both requested and allocated (they're always equal with hard caps)
        allocations.recruitment.requested = new_value;
        allocations.recruitment.allocated = new_value;

        debug!(
            "Recruitment allocation: {} → {} (capacity_cap={}, resource_cap={})",
            current, new_value, capacity_cap, resource_cap
        );
    }
}

/// System to apply training allocation adjustments (Input → Logic)
/// Reads AdjustTraining messages and updates ResourceAllocations
/// HARD CAP: requested value is immediately capped by all constraints
pub fn apply_training_adjustments(
    mut messages: MessageReader<AdjustTraining>,
    mut nations: Query<(&mut ResourceAllocations, &Workforce, &Stockpile, &Treasury)>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, workforce, stockpile, treasury)) = nations.get_mut(msg.nation)
        else {
            warn!("Cannot adjust training: nation not found");
            continue;
        };

        // Find or create training allocation for this skill level
        let existing_idx = allocations
            .training
            .iter()
            .position(|t| t.from_skill == msg.from_skill);

        let training_alloc = if let Some(idx) = existing_idx {
            &mut allocations.training[idx]
        } else {
            allocations.training.push(TrainingAllocation {
                from_skill: msg.from_skill,
                requested: 0,
                allocated: 0,
            });
            allocations.training.last_mut().unwrap()
        };

        // Calculate worker cap (how many workers of this skill exist)
        let worker_cap = workforce.count_by_skill(msg.from_skill);

        // Calculate resource cap (paper availability)
        let paper_available = stockpile.get_available(super::goods::Good::Paper);
        let paper_cap = paper_available / TrainingAllocation::inputs_per_unit()[0].1;

        // Calculate cash cap
        let cash_cap = (treasury.0.max(0) / TrainingAllocation::cash_per_unit()) as u32;

        // Hard cap: apply ALL constraints
        let max_possible = worker_cap.min(paper_cap).min(cash_cap);
        let new_value = msg.requested.min(max_possible);

        // Ignore if unchanged
        let current = training_alloc.requested;
        if new_value == current {
            debug!("Training allocation unchanged: {}", current);
            continue;
        }

        // Update both requested and allocated (they're always equal with hard caps)
        training_alloc.requested = new_value;
        training_alloc.allocated = new_value;

        debug!(
            "Training allocation ({:?}): {} → {} (worker_cap={}, paper_cap={}, cash_cap={})",
            msg.from_skill, current, new_value, worker_cap, paper_cap, cash_cap
        );
    }
}

/// System to apply production allocation adjustments (Input → Logic)
/// Reads AdjustProduction messages and updates ResourceAllocations
/// HARD CAP: target_output is immediately capped by all constraints
pub fn apply_production_adjustments(
    mut messages: MessageReader<AdjustProduction>,
    mut nations: Query<(&mut ResourceAllocations, &Stockpile, Option<&Workforce>)>,
    buildings: Query<&Building>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, stockpile, workforce_opt)) = nations.get_mut(msg.nation) else {
            warn!("Cannot adjust production: nation not found");
            continue;
        };

        let Ok(building) = buildings.get(msg.building) else {
            warn!("Cannot adjust production: building not found");
            continue;
        };

        // Get or create production allocation for this building
        let prod_alloc = allocations
            .production
            .entry(msg.building)
            .or_insert_with(|| ProductionAllocation {
                building_kind: building.kind,
                choice: super::production::ProductionChoice::UseCotton, // Default
                capacity: building.capacity,
                outputs: std::collections::HashMap::new(),
            });

        // Update choice if provided
        if let Some(choice) = msg.choice {
            prod_alloc.choice = choice;
        }

        // Update capacity and kind
        prod_alloc.capacity = building.capacity;
        prod_alloc.building_kind = building.kind;

        // Get current allocation for this specific output
        let current_alloc = prod_alloc
            .outputs
            .get(&msg.output_good)
            .map(|a| a.allocated)
            .unwrap_or(0);

        // Calculate max possible output for THIS specific output
        // Must account for:
        // 1. Building capacity (shared across all outputs)
        // 2. Input availability (accounting for other outputs' reservations)
        // 3. Labor availability

        // 1. Calculate remaining capacity (capacity - other outputs' allocations)
        let other_outputs_total: u32 = prod_alloc
            .outputs
            .iter()
            .filter(|(g, _)| **g != msg.output_good)
            .map(|(_, a)| a.allocated)
            .sum();
        let remaining_capacity = building.capacity.saturating_sub(other_outputs_total);
        let mut max_output = msg.target_output.min(remaining_capacity);

        // 2. Calculate input availability
        // For each input needed for this output, check what's available after subtracting
        // what's already reserved for other outputs

        // First, calculate what inputs are already reserved by other outputs
        let mut reserved_inputs = std::collections::HashMap::new();
        for (output_good, output_alloc) in &prod_alloc.outputs {
            if *output_good == msg.output_good {
                continue; // Skip the output we're currently adjusting
            }

            // Calculate inputs needed for this other output
            let inputs = calculate_inputs_for_output(
                building.kind,
                *output_good,
                output_alloc.allocated,
                prod_alloc.choice,
            );
            for (good, qty) in inputs {
                *reserved_inputs.entry(good).or_insert(0) += qty;
            }
        }

        // Now calculate max output based on available (non-reserved) inputs
        if building.kind == super::production::BuildingKind::TextileMill {
            // Special case: use SUM of cotton and wool
            let cotton_available = stockpile
                .get_available(super::goods::Good::Cotton)
                .saturating_sub(
                    *reserved_inputs
                        .get(&super::goods::Good::Cotton)
                        .unwrap_or(&0),
                );
            let wool_available = stockpile
                .get_available(super::goods::Good::Wool)
                .saturating_sub(*reserved_inputs.get(&super::goods::Good::Wool).unwrap_or(&0));
            let total_fiber = cotton_available + wool_available;
            let max_from_fiber = total_fiber / 2; // 2 fiber → 1 fabric
            max_output = max_output.min(max_from_fiber);
        } else {
            // Calculate inputs needed for the requested output
            let inputs_for_max = calculate_inputs_for_output(
                building.kind,
                msg.output_good,
                max_output,
                prod_alloc.choice,
            );

            for (good, qty_needed_for_max) in inputs_for_max {
                if qty_needed_for_max == 0 {
                    continue;
                }
                let total_available = stockpile.get_available(good);
                let already_reserved = *reserved_inputs.get(&good).unwrap_or(&0);
                let available = total_available.saturating_sub(already_reserved);

                let max_from_this_input = (available * max_output) / qty_needed_for_max;
                max_output = max_output.min(max_from_this_input);
            }
        }

        // 3. Check labor availability (1 labor per output unit)
        // Labor is shared across ALL outputs
        if let Some(workforce) = workforce_opt {
            let available_labor = workforce.available_labor();
            let other_labor_used = other_outputs_total;
            let remaining_labor = available_labor.saturating_sub(other_labor_used);
            max_output = max_output.min(remaining_labor);
        }

        // Hard cap the requested value
        let new_value = max_output;

        if new_value == current_alloc {
            debug!("Production allocation unchanged: {}", current_alloc);
            continue;
        }

        // Update the allocation for this specific output
        prod_alloc
            .outputs
            .entry(msg.output_good)
            .or_insert(super::allocation::OutputAllocation::default())
            .requested = new_value;
        prod_alloc
            .outputs
            .entry(msg.output_good)
            .or_insert(super::allocation::OutputAllocation::default())
            .allocated = new_value;

        debug!(
            "Production allocation ({:?} -> {:?}): {} → {} (remaining_capacity={})",
            building.kind, msg.output_good, current_alloc, new_value, remaining_capacity
        );
    }
}

/// Helper function to calculate inputs needed for a specific output
fn calculate_inputs_for_output(
    building_kind: super::production::BuildingKind,
    output_good: super::goods::Good,
    output_qty: u32,
    choice: super::production::ProductionChoice,
) -> Vec<(super::goods::Good, u32)> {
    use super::goods::Good;
    use super::production::{BuildingKind, ProductionChoice};

    if output_qty == 0 {
        return vec![];
    }

    match building_kind {
        BuildingKind::TextileMill => {
            // 2×Fiber (Cotton + Wool) → 1×Fabric
            // Special case: handled separately
            vec![]
        }

        BuildingKind::LumberMill => {
            // 2×Timber → 1×(Lumber|Paper)
            vec![(Good::Timber, output_qty * 2)]
        }

        BuildingKind::SteelMill => {
            // 1×Iron + 1×Coal → 1×Steel
            vec![(Good::Iron, output_qty), (Good::Coal, output_qty)]
        }

        BuildingKind::FoodProcessingCenter => {
            // 2×Grain + 1×Fruit + 1×(Livestock|Fish) → 2×CannedFood
            let meat_good = match choice {
                ProductionChoice::UseLivestock => Good::Livestock,
                ProductionChoice::UseFish => Good::Fish,
                _ => return vec![],
            };

            let batches = output_qty.div_ceil(2);
            vec![
                (Good::Grain, batches * 2),
                (Good::Fruit, batches),
                (meat_good, batches),
            ]
        }

        BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => vec![],
    }
}

/// System to finalize all allocations at turn end (before Processing phase)
/// Converts allocations to reservations and queued orders
pub fn finalize_allocations(
    turn: Res<TurnSystem>,
    mut nations: Query<(
        &ResourceAllocations,
        &mut Stockpile,
        &mut super::workforce::RecruitmentQueue,
        &mut super::workforce::TrainingQueue,
        &mut Treasury,
    )>,
    mut buildings: Query<(&mut ProductionSettings, &Building)>,
) {
    // Only run when transitioning to Processing
    if turn.phase != crate::turn_system::TurnPhase::Processing {
        return;
    }

    for (allocations, mut stockpile, mut recruit_queue, mut train_queue, _treasury) in
        nations.iter_mut()
    {
        // 1. Finalize recruitment allocations
        let r = &allocations.recruitment;
        if r.allocated > 0 {
            for (good, qty_per) in RecruitmentAllocation::inputs_per_unit() {
                let total_needed = qty_per * r.allocated;
                if !stockpile.reserve(good, total_needed) {
                    warn!(
                        "Failed to reserve {} {:?} for recruitment (need {})",
                        total_needed, good, total_needed
                    );
                }
            }
            recruit_queue.queued = r.allocated;
            info!("Finalized recruitment: {} workers queued", r.allocated);
        }

        // 2. Finalize training allocations
        for t in &allocations.training {
            if t.allocated == 0 {
                continue;
            }

            for (good, qty_per) in TrainingAllocation::inputs_per_unit() {
                let total_needed = qty_per * t.allocated;
                if !stockpile.reserve(good, total_needed) {
                    warn!(
                        "Failed to reserve {} {:?} for training (need {})",
                        total_needed, good, total_needed
                    );
                }
            }

            // Note: Treasury doesn't have reservations yet, so we just check total
            // The actual deduction happens in execute_training_orders
            train_queue.add_order(t.from_skill, t.allocated);

            info!(
                "Finalized training: {} workers ({:?} → {:?})",
                t.allocated,
                t.from_skill,
                t.from_skill.next_level()
            );
        }

        // 3. Finalize production allocations
        for (building_entity, prod_alloc) in &allocations.production {
            let total_allocated = prod_alloc.total_allocated();
            if total_allocated == 0 {
                continue;
            }

            // Update production settings (for now, just use first output's value)
            // TODO: ProductionSettings might need to support multi-output
            if let Ok((mut settings, _building)) = buildings.get_mut(*building_entity) {
                settings.choice = prod_alloc.choice;
                settings.target_output = total_allocated;
            }

            // Reserve inputs (now accounts for ALL outputs)
            for (good, qty) in prod_alloc.inputs_needed() {
                if !stockpile.reserve(good, qty) {
                    warn!(
                        "Failed to reserve {} {:?} for production (building {:?})",
                        qty, good, prod_alloc.building_kind
                    );
                }
            }

            info!(
                "Finalized production ({:?}): {} total output allocated across {} goods",
                prod_alloc.building_kind,
                total_allocated,
                prod_alloc.outputs.len()
            );
        }
    }
}

/// System to reset allocations at start of new PlayerTurn
/// This allows players to start fresh each turn
pub fn reset_allocations(turn: Res<TurnSystem>, mut nations: Query<&mut ResourceAllocations>) {
    // Only run at start of PlayerTurn
    if turn.phase != crate::turn_system::TurnPhase::PlayerTurn {
        return;
    }

    // Check if this is the first frame of PlayerTurn
    // (We need some way to avoid resetting every frame during PlayerTurn)
    // For now, we'll reset unconditionally and rely on UI to repopulate

    for mut allocations in nations.iter_mut() {
        *allocations = ResourceAllocations::default();
        debug!("Reset allocations for new turn");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::goods::Good;

    #[test]
    fn test_recruitment_allocation_caps() {
        // This would require a proper ECS setup, so just a placeholder
        // Real tests should use Bevy's App test utilities
    }
}
