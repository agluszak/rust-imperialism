use bevy::prelude::*;

use crate::economy::{
    allocation::Allocations,
    goods::Good,
    production::{BuildingKind, Buildings},
    reservation::ReservationSystem,
    stockpile::Stockpile,
    treasury::Treasury,
    workforce::{RecruitmentCapacity, types::*},
};
use crate::{
    map::province::Province,
    messages::{
        AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
    },
    orders::OrdersQueue,
    turn_system::TurnSystem,
};

// ============================================================================
// Production Adjustment System (Unit-by-Unit Reservations)
// ============================================================================

/// Apply production allocation adjustments using unit-by-unit reservations
/// Each +1 adds one ReservationId, each -1 removes one
pub fn apply_production_adjustments(
    mut messages: MessageReader<AdjustProduction>,
    mut orders: ResMut<OrdersQueue>,
) {
    for msg in messages.read() {
        orders.queue_production(*msg);
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

        BuildingKind::ClothingFactory => vec![(Good::Fabric, 2)],

        BuildingKind::FurnitureFactory => vec![(Good::Lumber, 2)],

        BuildingKind::MetalWorks => vec![(Good::Steel, 2)],

        BuildingKind::Refinery => vec![(Good::Oil, 2)],

        BuildingKind::Railyard => vec![(Good::Steel, 1), (Good::Lumber, 1)],

        BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => vec![],
    }
}

// ============================================================================
// Recruitment Adjustment System
// ============================================================================

/// Apply recruitment allocation adjustments using unit-by-unit reservations
pub fn apply_recruitment_adjustments(
    mut messages: MessageReader<AdjustRecruitment>,
    mut orders: ResMut<OrdersQueue>,
) {
    for msg in messages.read() {
        orders.queue_recruitment(*msg);
    }
}

// ============================================================================
// Training Adjustment System
// ============================================================================

/// Apply training allocation adjustments using unit-by-unit reservations
pub fn apply_training_adjustments(
    mut messages: MessageReader<AdjustTraining>,
    mut orders: ResMut<OrdersQueue>,
) {
    for msg in messages.read() {
        orders.queue_training(*msg);
    }
}

// ============================================================================
// Market Adjustment System
// ============================================================================

/// Apply market buy/sell allocation adjustments using reservations
pub fn apply_market_order_adjustments(
    mut messages: MessageReader<AdjustMarketOrder>,
    mut orders: ResMut<OrdersQueue>,
) {
    for msg in messages.read() {
        orders.queue_market(*msg);
    }
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
        Good::Hardware | Good::Armaments => BuildingKind::MetalWorks,
        Good::Fuel => BuildingKind::Refinery,
        Good::Transport => BuildingKind::Railyard,
        _ => {
            warn!(
                "Cannot determine building for output good: {:?}",
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
    for ((entity, _output), res_ids) in allocations.production.iter() {
        if *entity == msg.building {
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
        warn!("Cannot adjust market orders: nation not found");
        return;
    };

    match msg.kind {
        MarketInterest::Buy => {
            let wants_to_buy = msg.requested > 0;

            if wants_to_buy {
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

                if allocations.market_buy_interest.insert(msg.good) {
                    debug!("Set buy interest for {:?}", msg.good);
                }
            } else if allocations.market_buy_interest.remove(&msg.good) {
                debug!("Cleared buy interest for {:?}", msg.good);
            }
        }

        MarketInterest::Sell => {
            let target = msg.requested as usize;

            if target > 0 && allocations.market_buy_interest.remove(&msg.good) {
                debug!("Cleared buy interest for {:?}", msg.good);
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
                    debug!(
                        "Sell orders increased: {:?} {} → {} ({} added)",
                        msg.good,
                        current_count,
                        current_count + added,
                        added
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
pub fn finalize_allocations(
    _turn: Res<TurnSystem>,
    mut nations: Query<(
        &Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
        &mut crate::economy::workforce::RecruitmentQueue,
        &mut crate::economy::workforce::TrainingQueue,
    )>,
    mut buildings: Query<&mut crate::economy::production::ProductionSettings>,
) {
    // Note: This system only runs when TurnSystem changes AND phase == Processing
    // due to run_if conditions in lib.rs, so no need for phase check here

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

        // Log market buy interest - execution happens in dedicated market systems
        for good in &allocations.market_buy_interest {
            info!("Buy interest set for: {:?} (awaiting clearing)", good);
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
pub fn reset_allocations(
    _turn: Res<TurnSystem>,
    mut nations: Query<(
        &mut Allocations,
        &mut ReservationSystem,
        &mut Stockpile,
        &mut Workforce,
        &mut Treasury,
    )>,
) {
    // Note: This system only runs when TurnSystem changes AND phase == PlayerTurn
    // due to run_if conditions in lib.rs, so no need for phase check here

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
