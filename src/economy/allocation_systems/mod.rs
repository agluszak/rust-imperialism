use bevy::prelude::*;

use crate::economy::{
    allocation::Allocations,
    goods::Good,
    production::{building_for_output, Building, BuildingKind, Buildings},
    reservation::ReservationSystem,
    stockpile::Stockpile,
    treasury::Treasury,
    workforce::{types::*, RecruitmentCapacity},
};
use crate::{
    map::province::Province,
    messages::{
        AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
    },
    orders::OrdersQueue,
};

// ============================================================================
// Production Adjustment System (Unit-by-Unit Reservations)
// ============================================================================

/// Apply production allocation adjustments using unit-by-unit reservations
/// Each +1 adds one ReservationId, each -1 removes one
pub fn apply_production_adjustments(
    trigger: On<AdjustProduction>,
    mut orders: ResMut<OrdersQueue>,
) {
    orders.queue_production(*trigger.event());
}

/// Calculate inputs needed for one unit of production, intelligently choosing
/// based on stockpile availability (e.g., Cotton vs Wool, Fish vs Livestock)
pub(crate) fn calculate_inputs_for_one_unit(
    kind: BuildingKind,
    output: Good,
    stockpile: &Stockpile,
) -> Vec<(Good, u32)> {
    use crate::economy::production::production_recipe;

    if let Some(recipe) = production_recipe(kind) {
        if let Some(variant) = recipe.best_variant_for_output(output, stockpile) {
            return variant
                .inputs()
                .iter()
                .map(|i| (i.good, i.amount))
                .collect();
        }
    }

    vec![]
}

// ============================================================================
// Recruitment Adjustment System
// ============================================================================

/// Apply recruitment allocation adjustments using unit-by-unit reservations
pub fn apply_recruitment_adjustments(
    trigger: On<AdjustRecruitment>,
    mut orders: ResMut<OrdersQueue>,
) {
    orders.queue_recruitment(*trigger.event());
}

// ============================================================================
// Training Adjustment System
// ============================================================================

/// Apply training allocation adjustments using unit-by-unit reservations
pub fn apply_training_adjustments(trigger: On<AdjustTraining>, mut orders: ResMut<OrdersQueue>) {
    orders.queue_training(*trigger.event());
}

// ============================================================================
// Market Adjustment System
// ============================================================================

/// Apply market buy/sell allocation adjustments using reservations
pub fn apply_market_order_adjustments(
    trigger: On<AdjustMarketOrder>,
    mut orders: ResMut<OrdersQueue>,
) {
    orders.queue_market(*trigger.event());
}

pub fn execute_queued_production_orders(
    mut orders: ResMut<OrdersQueue>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
    )>,
    buildings_query: Query<&Buildings>,
) {
    let queued = orders.take_production();
    if queued.is_empty() {
        return;
    }

    for order in queued {
        process_production_adjustment(order, &mut nations, &buildings_query);
    }
}

fn process_production_adjustment(
    msg: AdjustProduction,
    nations: &mut Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
    )>,
    buildings_query: &Query<&Buildings>,
) {
    let Ok((mut allocations, mut reservations, mut stockpile, mut workforce)) =
        nations.get_mut(msg.nation.entity())
    else {
        warn!("Cannot adjust production: nation not found");
        return;
    };

    let Ok(buildings_collection) = buildings_query.get(msg.building) else {
        warn!("Cannot adjust production: buildings not found");
        return;
    };

    let building_kind = match msg.output_good {
        Good::Fabric => BuildingKind::TextileMill,
        Good::Paper | Good::Lumber => BuildingKind::LumberMill,
        Good::Steel => BuildingKind::SteelMill,
        Good::CannedFood => BuildingKind::FoodProcessingCenter,
        Good::Clothing => BuildingKind::ClothingFactory,
        Good::Furniture => BuildingKind::FurnitureFactory,
        Good::Hardware | Good::Arms => BuildingKind::MetalWorks,
        Good::Fuel => BuildingKind::Refinery,
        Good::Transport => BuildingKind::Railyard,
        _ => {
            warn!(
                "Cannot determine building for output good: {:?}. Note: Ships are constructed separately.",
                msg.output_good
            );
            return;
        }
    };

    let Some(building) = buildings_collection.get(building_kind) else {
        warn!(
            "Cannot adjust production: building kind not found: {:?}",
            building_kind
        );
        return;
    };

    let key = (msg.building, msg.output_good);
    let current_count = allocations.production_count(msg.building, msg.output_good);

    let mut total_building_production = 0u32;
    for ((entity, output), res_ids) in allocations.production.iter() {
        if *entity != msg.building {
            continue;
        }

        let Some(output_building_kind) = building_for_output(*output) else {
            continue;
        };

        if output_building_kind == building_kind {
            total_building_production += res_ids.len() as u32;
        }
    }

    let current_count_u32 = current_count as u32;
    let other_outputs = total_building_production.saturating_sub(current_count_u32);
    let remaining_capacity = building.capacity.saturating_sub(other_outputs);

    let target = msg.target_output.min(remaining_capacity) as usize;

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
    } else if target > current_count {
        let to_add = target - current_count;
        let vec = allocations.production.entry(key).or_default();
        let mut added = 0;

        for _ in 0..to_add {
            let inputs_per_unit =
                calculate_inputs_for_one_unit(building.kind, msg.output_good, &stockpile);

            if let Some(res_id) = reservations.try_reserve(
                inputs_per_unit.clone(),
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut Treasury::new(0),
            ) {
                vec.push(res_id);
                added += 1;
            } else {
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

pub fn execute_queued_recruitment_orders(
    mut orders: ResMut<OrdersQueue>,
    mut nations: Query<(&mut Allocations, &mut ReservationSystem, &mut Stockpile)>,
    provinces: Query<&Province>,
    recruitment_capacity: Query<&RecruitmentCapacity>,
) {
    let queued = orders.take_recruitment();
    if queued.is_empty() {
        return;
    }

    for order in queued {
        process_recruitment_adjustment(order, &mut nations, &provinces, &recruitment_capacity);
    }
}

fn process_recruitment_adjustment(
    msg: AdjustRecruitment,
    nations: &mut Query<(&mut Allocations, &mut ReservationSystem, &mut Stockpile)>,
    provinces: &Query<&Province>,
    recruitment_capacity: &Query<&RecruitmentCapacity>,
) {
    let Ok((mut allocations, mut reservations, mut stockpile)) =
        nations.get_mut(msg.nation.entity())
    else {
        warn!("Cannot adjust recruitment: nation not found");
        return;
    };

    let province_count = provinces
        .iter()
        .filter(|p| p.owner == Some(msg.nation.entity()))
        .count() as u32;

    let capacity_upgraded = recruitment_capacity
        .get(msg.nation.entity())
        .map(|c| c.upgraded)
        .unwrap_or(false);

    let capacity_cap = if capacity_upgraded {
        province_count / 3
    } else {
        province_count / 4
    };

    let current_count = allocations.recruitment_count();
    let target = msg.requested.min(capacity_cap) as usize;

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
    } else if target > current_count {
        let to_add = target - current_count;

        let inputs = vec![
            (Good::CannedFood, 1),
            (Good::Clothing, 1),
            (Good::Furniture, 1),
        ];

        let mut added = 0;

        for _ in 0..to_add {
            if let Some(res_id) = reservations.try_reserve(
                inputs.clone(),
                0,
                0,
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

pub fn execute_queued_training_orders(
    mut orders: ResMut<OrdersQueue>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &Workforce,
        &mut Treasury,
    )>,
) {
    let queued = orders.take_training();
    if queued.is_empty() {
        return;
    }

    for order in queued {
        process_training_adjustment(order, &mut nations);
    }
}

fn process_training_adjustment(
    msg: AdjustTraining,
    nations: &mut Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &Workforce,
        &mut Treasury,
    )>,
) {
    let Ok((mut allocations, mut reservations, mut stockpile, workforce, mut treasury)) =
        nations.get_mut(msg.nation.entity())
    else {
        warn!("Cannot adjust training: nation not found");
        return;
    };

    let worker_cap = workforce.count_by_skill(msg.from_skill);
    let current_count = allocations.training_count(msg.from_skill);
    let target = msg.requested.min(worker_cap) as usize;

    if target < current_count {
        let to_remove = current_count - target;
        let vec = allocations.training.entry(msg.from_skill).or_default();

        for _ in 0..to_remove {
            if let Some(res_id) = vec.pop() {
                reservations.release(res_id, &mut stockpile, &mut Workforce::new(), &mut treasury);
            }
        }

        debug!(
            "Training decreased: {:?} {} → {}",
            msg.from_skill, current_count, target
        );
    } else if target > current_count {
        let to_add = target - current_count;
        let inputs = vec![(Good::Paper, 1)];
        const TRAINING_COST: u32 = 100;

        let vec = allocations.training.entry(msg.from_skill).or_default();
        let mut added = 0;

        for _ in 0..to_add {
            if let Some(res_id) = reservations.try_reserve(
                inputs.clone(),
                0,
                TRAINING_COST,
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

pub fn execute_queued_market_orders(
    mut orders: ResMut<OrdersQueue>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
    )>,
) {
    let queued = orders.take_market();
    if queued.is_empty() {
        return;
    }

    for order in queued {
        process_market_adjustment(order, &mut nations);
    }
}

pub fn execute_queued_transport_orders(mut orders: ResMut<OrdersQueue>, mut commands: Commands) {
    let queued = orders.take_transport();
    if queued.is_empty() {
        return;
    }

    for order in queued {
        commands.trigger(order);
    }
}

fn process_market_adjustment(
    msg: AdjustMarketOrder,
    nations: &mut Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
    )>,
) {
    let Ok((mut allocations, mut reservations, mut stockpile, mut workforce, mut treasury)) =
        nations.get_mut(msg.nation.entity())
    else {
        // Silently skip messages for despawned nations
        return;
    };

    match msg.kind {
        MarketInterest::Buy => {
            // Buy interest is boolean (requested > 0 means interested, 0 means not interested)
            let wants_buy = msg.requested > 0;

            if wants_buy {
                // Clear any conflicting sell orders first
                if let Some(sell_orders) = allocations.market_sells.get_mut(&msg.good) {
                    let cleared_count = sell_orders.len();
                    while let Some(res_id) = sell_orders.pop() {
                        reservations.release(res_id, &mut stockpile, &mut workforce, &mut treasury);
                    }
                    if cleared_count > 0 {
                        debug!(
                            "Cleared {} sell orders for {:?} (switching to buy interest)",
                            cleared_count, msg.good
                        );
                    }
                }

                // Express buy interest (boolean)
                if allocations.market_buys.insert(msg.good) {
                    debug!("Set buy interest for {:?}", msg.good);
                }
            } else if allocations.market_buys.remove(&msg.good) {
                debug!("Cleared buy interest for {:?}", msg.good);
            }
        }

        MarketInterest::Sell => {
            let target = msg.requested as usize;

            if target > 0 && allocations.market_buys.remove(&msg.good) {
                debug!(
                    "Cleared buy interest for {:?} (switching to sell offers)",
                    msg.good
                );
            }

            let vec = allocations.market_sells.entry(msg.good).or_default();
            let current_count = vec.len();

            if target < current_count {
                let to_remove = current_count - target;

                for _ in 0..to_remove {
                    if let Some(res_id) = vec.pop() {
                        reservations.release(res_id, &mut stockpile, &mut workforce, &mut treasury);
                    }
                }
            } else if target > current_count {
                let to_add = target - current_count;
                let mut added = 0;

                for _ in 0..to_add {
                    if let Some(res_id) = reservations.try_reserve(
                        vec![(msg.good, 1)],
                        0,
                        0,
                        &mut stockpile,
                        &mut workforce,
                        &mut treasury,
                    ) {
                        vec.push(res_id);
                        added += 1;
                    } else {
                        break;
                    }
                }

                if added > 0 {
                    info!(
                        "Sell orders increased: {:?} {} → {} ({} added, nation: {:?})",
                        msg.good,
                        current_count,
                        current_count + added,
                        added,
                        msg.nation.entity()
                    );
                } else if to_add > 0 {
                    info!(
                        "Failed to create sell orders for {:?}: wanted {} but could only add {} (insufficient available stock)",
                        msg.good, to_add, added
                    );
                }
            }
        }
    }
}

// ============================================================================
// Turn Management Systems
// ============================================================================

/// Finalize allocations at turn end (when entering Processing phase)
/// Consumes reservations and queues orders for execution
/// NOTE: Registered via OnEnter(TurnPhase::Processing), so no phase check needed.
pub fn finalize_allocations(
    mut nations: Query<(
        &Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
        &mut crate::economy::workforce::RecruitmentQueue,
        &mut crate::economy::workforce::TrainingQueue,
    )>,
    mut buildings: Query<(
        &mut crate::economy::production::ProductionSettings,
        &Building,
    )>,
) {
    use crate::economy::production::production_recipe;

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
                // Determine output amount per batch
                let output_amount =
                    if let Ok((_, building)) = buildings.get(*building_entity) {
                        if let Some(recipe) = production_recipe(building.kind) {
                            let variants = recipe.variants_for_output(*output_good);
                            if let Some(info) = variants.first() {
                                info.variant.primary_output_amount()
                            } else {
                                1
                            }
                        } else {
                            1
                        }
                    } else {
                        1
                    };

                // Consume all production reservations
                for res_id in res_ids {
                    reservations.consume(*res_id, &mut stockpile, &mut workforce, &mut treasury);
                }

                // Add output to stockpile
                let total_output = (production_count as u32) * output_amount;
                stockpile.add(*output_good, total_output);

                info!(
                    "Production executed: building {:?}, output {:?} x {}",
                    building_entity, output_good, total_output
                );

                // Update production settings
                if let Ok((mut settings, _)) = buildings.get_mut(*building_entity) {
                    settings.target_output = total_output;
                    info!(
                        "Finalized production: building {:?}, output {:?}, target {}",
                        building_entity, output_good, total_output
                    );
                }
            }
        }

        // Log market buy interest - execution happens in dedicated market systems
        for good in &allocations.market_buys {
            info!("Buy interest queued: {:?} (awaiting clearing)", good);
        }

        for (good, res_ids) in &allocations.market_sells {
            if !res_ids.is_empty() {
                info!(
                    "Queued market sell offers: {} × {:?} (awaiting clearing)",
                    res_ids.len(),
                    good
                );
            }
        }
    }
}

/// Reset allocations at start of PlayerTurn
/// Releases all reservations and clears allocation structures
/// NOTE: Registered via OnEnter(TurnPhase::PlayerTurn), so no phase check needed.
pub fn reset_allocations(
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
    )>,
) {
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

        // Buy interest has no reservations to release (it's just a flag)

        // Release market sell reservations (return goods)
        for (_good, res_ids) in allocations.market_sells.iter() {
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
mod tests;
