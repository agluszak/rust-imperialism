use bevy::prelude::*;

use super::{
    allocation::{AdjustProduction, AdjustRecruitment, AdjustTraining, Allocations},
    goods::Good,
    production::{BuildingKind, Buildings},
    reservation::ReservationSystem,
    stockpile::Stockpile,
    treasury::Treasury,
    workforce::{RecruitmentCapacity, types::*},
};
use crate::province::Province;

// ============================================================================
// Production Adjustment System (Unit-by-Unit Reservations)
// ============================================================================

/// Apply production allocation adjustments using unit-by-unit reservations
/// Each +1 adds one ReservationId, each -1 removes one
pub fn apply_production_adjustments(
    mut messages: MessageReader<AdjustProduction>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
    )>,
    buildings_query: Query<&Buildings>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, mut reservations, mut stockpile, mut workforce)) =
            nations.get_mut(msg.nation)
        else {
            warn!("Cannot adjust production: nation not found");
            continue;
        };

        // Get the buildings collection and find the matching building for this output
        let Ok(buildings_collection) = buildings_query.get(msg.building) else {
            warn!("Cannot adjust production: buildings not found");
            continue;
        };

        // Infer building kind from output_good
        let building_kind = match msg.output_good {
            Good::Fabric => BuildingKind::TextileMill,
            Good::Paper | Good::Lumber => BuildingKind::LumberMill,
            Good::Steel => BuildingKind::SteelMill,
            Good::CannedFood => BuildingKind::FoodProcessingCenter,
            _ => {
                warn!(
                    "Cannot determine building for output good: {:?}",
                    msg.output_good
                );
                continue;
            }
        };

        let Some(building) = buildings_collection.get(building_kind) else {
            warn!(
                "Cannot adjust production: building kind not found: {:?}",
                building_kind
            );
            continue;
        };

        let key = (msg.building, msg.output_good);
        let current_count = allocations.production_count(msg.building, msg.output_good);

        // Calculate total current production for this building across ALL outputs
        let mut total_building_production = 0u32;
        for ((entity, _output), res_ids) in allocations.production.iter() {
            if *entity == msg.building {
                total_building_production += res_ids.len() as u32;
            }
        }

        // Calculate remaining capacity (excluding current allocation for this specific output)
        let current_count_u32 = current_count as u32;
        let other_outputs = total_building_production - current_count_u32;
        let remaining_capacity = building.capacity.saturating_sub(other_outputs);

        // Cap target at remaining capacity
        let target = msg.target_output.min(remaining_capacity) as usize;

        // Decrease: remove reservations
        if target < current_count {
            let to_remove = current_count - target;
            let vec = allocations.production.entry(key).or_default();

            for _ in 0..to_remove {
                if let Some(res_id) = vec.pop() {
                    reservations.release(
                        res_id,
                        &mut stockpile,
                        &mut workforce,
                        &mut Treasury::new(0),
                    );
                }
            }

            debug!(
                "Production decreased: {:?} {:?} {} → {}",
                building.kind, msg.output_good, current_count, target
            );
        }
        // Increase: try to add reservations one by one
        else if target > current_count {
            let to_add = target - current_count;

            let vec = allocations.production.entry(key).or_default();
            let mut added = 0;

            for _ in 0..to_add {
                // Calculate inputs per unit dynamically (checks stockpile availability)
                let inputs_per_unit =
                    calculate_inputs_for_one_unit(building.kind, msg.output_good, &stockpile);

                debug!(
                    "Attempting to reserve for {:?} {:?}: inputs={:?}, labor=1, available_labor={}, labor_pool={:?}",
                    building.kind,
                    msg.output_good,
                    &inputs_per_unit,
                    workforce.available_labor(),
                    workforce.labor_pool
                );

                // Try to reserve for ONE unit
                if let Some(res_id) = reservations.try_reserve(
                    inputs_per_unit.clone(),
                    1, // 1 labor per unit
                    0, // no money
                    &mut stockpile,
                    &mut workforce,
                    &mut Treasury::new(0),
                ) {
                    vec.push(res_id);
                    added += 1;
                    debug!("Reservation successful");
                } else {
                    // Can't reserve more, stop trying
                    debug!(
                        "Reservation failed - stockpile: [{}], workforce: untrained={}, trained={}, expert={}, labor_pool.available={}",
                        inputs_per_unit.iter().map(|(g, amt)| format!("{:?}={}/{}", g, stockpile.get_available(*g), amt)).collect::<Vec<_>>().join(", "),
                        workforce.untrained_count(),
                        workforce.trained_count(),
                        workforce.expert_count(),
                        workforce.labor_pool.available()
                    );
                    break;
                }
            }

            if added > 0 {
                debug!(
                    "Production increased: {:?} {:?} {} → {} ({} added)",
                    building.kind,
                    msg.output_good,
                    current_count,
                    current_count + added,
                    added
                );
            } else if to_add > 0 {
                debug!(
                    "Production increase failed: {:?} {:?} - insufficient resources",
                    building.kind, msg.output_good
                );
            }
        }
    }
}

/// Calculate inputs needed for one unit of production, intelligently choosing
/// based on stockpile availability (e.g., Cotton vs Wool, Fish vs Livestock)
pub(crate) fn calculate_inputs_for_one_unit(
    kind: BuildingKind,
    _output: Good,
    stockpile: &Stockpile,
) -> Vec<(Good, u32)> {
    match kind {
        BuildingKind::TextileMill => {
            // 2 fiber → 1 fabric
            // Intelligently pick Cotton or Wool based on availability
            let cotton_available = stockpile.get_available(Good::Cotton);
            let wool_available = stockpile.get_available(Good::Wool);

            // Prefer whichever has more available (at least 2 units needed)
            let fiber = if cotton_available >= 2 {
                Good::Cotton
            } else if wool_available >= 2 {
                Good::Wool
            } else if cotton_available > wool_available {
                Good::Cotton
            } else {
                Good::Wool
            };
            vec![(fiber, 2)]
        }

        BuildingKind::LumberMill => {
            // 2 timber → 1 output (Lumber or Paper)
            vec![(Good::Timber, 2)]
        }

        BuildingKind::SteelMill => {
            // 1 iron + 1 coal → 1 steel
            vec![(Good::Iron, 1), (Good::Coal, 1)]
        }

        BuildingKind::FoodProcessingCenter => {
            // 2 Grain + 1 Fruit + 1 Meat → 2 CannedFood
            // Per unit: 2 Grain, 1 Fruit, 1 Meat (produces 2 units)
            // Intelligently pick Fish or Livestock based on availability
            let fish_available = stockpile.get_available(Good::Fish);
            let livestock_available = stockpile.get_available(Good::Livestock);

            let meat = if fish_available >= 1 {
                Good::Fish
            } else if livestock_available >= 1 {
                Good::Livestock
            } else if fish_available > 0 {
                Good::Fish
            } else {
                Good::Livestock
            };

            vec![(Good::Grain, 2), (Good::Fruit, 1), (meat, 1)]
        }

        BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => vec![],
    }
}

// ============================================================================
// Recruitment Adjustment System
// ============================================================================

/// Apply recruitment allocation adjustments using unit-by-unit reservations
pub fn apply_recruitment_adjustments(
    mut messages: MessageReader<AdjustRecruitment>,
    mut nations: Query<(&mut Allocations, &mut ReservationSystem, &mut Stockpile)>,
    provinces: Query<&Province>,
    recruitment_capacity: Query<&RecruitmentCapacity>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, mut reservations, mut stockpile)) = nations.get_mut(msg.nation)
        else {
            warn!("Cannot adjust recruitment: nation not found");
            continue;
        };

        // Calculate capacity cap
        let province_count = provinces
            .iter()
            .filter(|p| p.owner == Some(msg.nation))
            .count() as u32;

        let capacity_upgraded = recruitment_capacity
            .get(msg.nation)
            .map(|c| c.upgraded)
            .unwrap_or(false);

        let capacity_cap = if capacity_upgraded {
            province_count / 3
        } else {
            province_count / 4
        };

        let current_count = allocations.recruitment_count();
        let target = msg.requested.min(capacity_cap) as usize;

        // Decrease: remove reservations
        if target < current_count {
            let to_remove = current_count - target;

            for _ in 0..to_remove {
                if let Some(res_id) = allocations.recruitment.pop() {
                    reservations.release(
                        res_id,
                        &mut stockpile,
                        &mut Workforce::new(),
                        &mut Treasury::new(0),
                    );
                }
            }

            debug!("Recruitment decreased: {} → {}", current_count, target);
        }
        // Increase: try to add reservations
        else if target > current_count {
            let to_add = target - current_count;

            // Each worker needs: 1 CannedFood, 1 Clothing, 1 Furniture
            let inputs = vec![
                (Good::CannedFood, 1),
                (Good::Clothing, 1),
                (Good::Furniture, 1),
            ];

            let mut added = 0;

            for _ in 0..to_add {
                if let Some(res_id) = reservations.try_reserve(
                    inputs.clone(),
                    0, // no labor
                    0, // no money
                    &mut stockpile,
                    &mut Workforce::new(),
                    &mut Treasury::new(0),
                ) {
                    allocations.recruitment.push(res_id);
                    added += 1;
                } else {
                    break;
                }
            }

            if added > 0 {
                debug!(
                    "Recruitment increased: {} → {} ({} added)",
                    current_count,
                    current_count + added,
                    added
                );
            }
        }
    }
}

// ============================================================================
// Training Adjustment System
// ============================================================================

/// Apply training allocation adjustments using unit-by-unit reservations
pub fn apply_training_adjustments(
    mut messages: MessageReader<AdjustTraining>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &Workforce,
        &mut Treasury,
    )>,
) {
    for msg in messages.read() {
        let Ok((mut allocations, mut reservations, mut stockpile, workforce, mut treasury)) =
            nations.get_mut(msg.nation)
        else {
            warn!("Cannot adjust training: nation not found");
            continue;
        };

        // Calculate worker cap
        let worker_cap = workforce.count_by_skill(msg.from_skill);

        let current_count = allocations.training_count(msg.from_skill);
        let target = msg.requested.min(worker_cap) as usize;

        // Decrease: remove reservations
        if target < current_count {
            let to_remove = current_count - target;
            let vec = allocations.training.entry(msg.from_skill).or_default();

            for _ in 0..to_remove {
                if let Some(res_id) = vec.pop() {
                    reservations.release(
                        res_id,
                        &mut stockpile,
                        &mut Workforce::new(),
                        &mut treasury,
                    );
                }
            }

            debug!(
                "Training decreased: {:?} {} → {}",
                msg.from_skill, current_count, target
            );
        }
        // Increase: try to add reservations
        else if target > current_count {
            let to_add = target - current_count;

            // Each training needs: 1 Paper, $100
            let inputs = vec![(Good::Paper, 1)];
            const TRAINING_COST: u32 = 100;

            let vec = allocations.training.entry(msg.from_skill).or_default();
            let mut added = 0;

            for _ in 0..to_add {
                if let Some(res_id) = reservations.try_reserve(
                    inputs.clone(),
                    0,             // no labor
                    TRAINING_COST, // $100
                    &mut stockpile,
                    &mut Workforce::new(),
                    &mut treasury,
                ) {
                    vec.push(res_id);
                    added += 1;
                } else {
                    break;
                }
            }

            if added > 0 {
                debug!(
                    "Training increased: {:?} {} → {} ({} added)",
                    msg.from_skill,
                    current_count,
                    current_count + added,
                    added
                );
            }
        }
    }
}

// ============================================================================
// Turn Management Systems
// ============================================================================

/// Finalize allocations at turn end (when entering Processing phase)
/// Consumes reservations and queues orders for execution
pub fn finalize_allocations(
    turn: Res<crate::turn_system::TurnSystem>,
    mut nations: Query<(
        &Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
        &mut super::workforce::RecruitmentQueue,
        &mut super::workforce::TrainingQueue,
    )>,
    mut buildings: Query<&mut super::production::ProductionSettings>,
) {
    use crate::turn_system::TurnPhase;

    // Only run when transitioning to Processing
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (
        allocations,
        mut reservations,
        mut stockpile,
        mut workforce,
        mut treasury,
        mut recruit_queue,
        mut train_queue,
    ) in nations.iter_mut()
    {
        // 1. Finalize recruitment allocations
        let recruitment_count = allocations.recruitment_count();
        if recruitment_count > 0 {
            // Consume all recruitment reservations
            for res_id in &allocations.recruitment {
                reservations.consume(*res_id, &mut stockpile, &mut workforce, &mut treasury);
            }
            recruit_queue.queued = recruitment_count as u32;
            info!(
                "Finalized recruitment: {} workers queued",
                recruitment_count
            );
        }

        // 2. Finalize training allocations
        for (from_skill, res_ids) in &allocations.training {
            let training_count = res_ids.len();
            if training_count > 0 {
                // Consume all training reservations for this skill level
                for res_id in res_ids {
                    reservations.consume(*res_id, &mut stockpile, &mut workforce, &mut treasury);
                }
                train_queue.add_order(*from_skill, training_count as u32);
                info!(
                    "Finalized training: {} workers ({:?} → {:?})",
                    training_count,
                    from_skill,
                    from_skill.next_level()
                );
            }
        }

        // 3. Finalize production allocations
        for ((building_entity, output_good), res_ids) in &allocations.production {
            let production_count = res_ids.len();
            if production_count > 0 {
                // Consume all production reservations
                for res_id in res_ids {
                    reservations.consume(*res_id, &mut stockpile, &mut workforce, &mut treasury);
                }

                // Update production settings
                if let Ok(mut settings) = buildings.get_mut(*building_entity) {
                    settings.target_output = production_count as u32;
                    info!(
                        "Finalized production: building {:?}, output {:?}, target {}",
                        building_entity, output_good, production_count
                    );
                }
            }
        }
    }
}

/// Reset allocations at start of PlayerTurn
/// Releases all reservations and clears allocation structures
pub fn reset_allocations(
    turn: Res<crate::turn_system::TurnSystem>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
    )>,
) {
    use crate::turn_system::TurnPhase;

    // Only run at start of PlayerTurn
    if turn.phase != TurnPhase::PlayerTurn {
        return;
    }

    for (mut allocations, mut reservations, mut stockpile, mut workforce, mut treasury) in
        nations.iter_mut()
    {
        // Release all production reservations
        for (_key, res_ids) in allocations.production.iter() {
            for res_id in res_ids {
                reservations.release(*res_id, &mut stockpile, &mut workforce, &mut treasury);
            }
        }

        // Release all recruitment reservations
        for res_id in &allocations.recruitment {
            reservations.release(*res_id, &mut stockpile, &mut workforce, &mut treasury);
        }

        // Release all training reservations
        for (_skill, res_ids) in allocations.training.iter() {
            for res_id in res_ids {
                reservations.release(*res_id, &mut stockpile, &mut workforce, &mut treasury);
            }
        }

        // Clear allocations
        *allocations = Allocations::default();
        debug!("Reset allocations for new turn");
    }
}

#[cfg(test)]
#[path = "allocation_systems_tests.rs"]
mod tests;
