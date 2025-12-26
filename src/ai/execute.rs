//! AI order execution.
//!
//! This module converts AI plans into concrete game orders (messages).

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::ai::markers::AiNation;
use crate::ai::planner::{CivilianTask, NationPlan, plan_nation};
use crate::ai::snapshot::AiSnapshot;
use crate::civilians::types::CivilianOrderKind;
use crate::economy::NationInstance;
use crate::economy::production::Buildings;
use crate::messages::civilians::CivilianCommand;
use crate::messages::{AdjustMarketOrder, AdjustProduction, HireCivilian, MarketInterest};

/// Main AI execution system - runs once per EnemyTurn.
///
/// This system:
/// 1. Reads the AI snapshot
/// 2. Generates a plan for each AI nation
/// 3. Sends orders to execute the plan
pub fn execute_ai_turn(
    snapshot: Res<AiSnapshot>,
    ai_nations: Query<(NationInstance, &Buildings), With<AiNation>>,
    mut civilian_commands: MessageWriter<CivilianCommand>,
    mut market_orders: MessageWriter<AdjustMarketOrder>,
    mut hire_messages: MessageWriter<HireCivilian>,
    mut production_orders: MessageWriter<AdjustProduction>,
    mut transport_orders: MessageWriter<crate::economy::transport::TransportAdjustAllocation>,
) {
    for (nation, buildings) in ai_nations.iter() {
        let Some(nation_snapshot) = snapshot.get_nation(nation.entity()) else {
            continue;
        };

        // Generate the plan
        let plan = plan_nation(nation_snapshot, &snapshot);

        // Execute the plan
        execute_plan(
            &plan,
            nation,
            buildings,
            &mut civilian_commands,
            &mut market_orders,
            &mut hire_messages,
            &mut production_orders,
            &mut transport_orders,
        );
    }
}

fn execute_plan(
    plan: &NationPlan,
    nation: NationInstance,
    _buildings: &Buildings,
    civilian_commands: &mut MessageWriter<CivilianCommand>,
    market_orders: &mut MessageWriter<AdjustMarketOrder>,
    hire_messages: &mut MessageWriter<HireCivilian>,
    production_orders: &mut MessageWriter<AdjustProduction>,
    transport_orders: &mut MessageWriter<crate::economy::transport::TransportAdjustAllocation>,
) {
    // Send civilian orders
    for (&civilian_entity, task) in &plan.civilian_tasks {
        if let Some(order) = task_to_order(task) {
            civilian_commands.write(CivilianCommand {
                civilian: civilian_entity,
                order,
            });
        }
    }

    // Send market buy orders
    for (good, qty) in &plan.market_buys {
        market_orders.write(AdjustMarketOrder {
            nation,
            good: *good,
            kind: MarketInterest::Buy,
            requested: *qty,
        });
    }

    // Send market sell orders
    for (good, qty) in &plan.market_sells {
        market_orders.write(AdjustMarketOrder {
            nation,
            good: *good,
            kind: MarketInterest::Sell,
            requested: *qty,
        });
    }

    // Send hire orders
    for kind in &plan.civilians_to_hire {
        hire_messages.write(HireCivilian {
            nation,
            kind: *kind,
        });
    }

    // Send production orders
    for order in &plan.production_orders {
        production_orders.write(AdjustProduction {
            nation,
            building: order.building,
            output_good: order.output,
            target_output: order.qty,
        });
    }

    // Send transport allocation orders
    for (commodity, requested) in &plan.transport_allocations {
        transport_orders.write(crate::economy::transport::TransportAdjustAllocation {
            nation: nation.entity(),
            commodity: *commodity,
            requested: *requested,
        });
    }
}

fn task_to_order(task: &CivilianTask) -> Option<CivilianOrderKind> {
    match task {
        CivilianTask::BuildRailTo { target } => Some(CivilianOrderKind::BuildRail { to: *target }),
        CivilianTask::BuildDepot => Some(CivilianOrderKind::BuildDepot),
        CivilianTask::ImproveTile { target } => {
            // Use the ImproveTile order - the civilian's kind determines improvement type
            Some(CivilianOrderKind::ImproveTile { to: *target })
        }
        CivilianTask::ProspectTile { target } => Some(CivilianOrderKind::Prospect { to: *target }),
        CivilianTask::MoveTo { target } => Some(CivilianOrderKind::Move { to: *target }),
        CivilianTask::Idle => None, // Skip turn, no order needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civilians::types::CivilianOrderKind;
    use bevy_ecs_tilemap::prelude::TilePos;

    #[test]
    fn test_task_to_order_conversion() {
        let target = TilePos::new(5, 5);

        assert!(matches!(
            task_to_order(&CivilianTask::BuildRailTo { target }),
            Some(CivilianOrderKind::BuildRail { .. })
        ));

        assert!(matches!(
            task_to_order(&CivilianTask::BuildDepot),
            Some(CivilianOrderKind::BuildDepot)
        ));

        assert!(matches!(
            task_to_order(&CivilianTask::MoveTo { target }),
            Some(CivilianOrderKind::Move { .. })
        ));

        assert!(task_to_order(&CivilianTask::Idle).is_none());
    }
}
