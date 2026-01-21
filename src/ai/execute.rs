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
use crate::economy::buildings::Buildings;
use crate::messages::civilians::CivilianCommand;
use crate::messages::{AdjustMarketOrder, AdjustProduction, HireCivilian, MarketInterest};

/// Main AI execution system - runs once per EnemyTurn.
///
/// This system:
/// 1. Reads the AI snapshot
/// 2. Generates a plan for each AI nation
/// 3. Sends orders to execute the plan
pub fn execute_ai_turn(
    mut commands: Commands,
    snapshot: Res<AiSnapshot>,
    ai_nations: Query<(NationInstance, &Buildings), With<AiNation>>,
    mut civilian_commands: MessageWriter<CivilianCommand>,
    mut hire_messages: MessageWriter<HireCivilian>,
) {
    for (nation, buildings) in ai_nations.iter() {
        let Some(nation_snapshot) = snapshot.get_nation(nation.entity()) else {
            continue;
        };

        // Generate the plan
        let plan = plan_nation(nation_snapshot, &snapshot);

        // Execute the plan
        execute_plan(
            &mut commands,
            &snapshot,
            &plan,
            nation,
            buildings,
            &mut civilian_commands,
            &mut hire_messages,
        );
    }
}

fn execute_plan(
    commands: &mut Commands,
    snapshot: &AiSnapshot,
    plan: &NationPlan,
    nation: NationInstance,
    _buildings: &Buildings,
    civilian_commands: &mut MessageWriter<CivilianCommand>,
    hire_messages: &mut MessageWriter<HireCivilian>,
) {
    // Build map of current positions for this nation's civilians
    // This allows us to know "who is at tile X" to establish dependencies
    let mut current_positions = std::collections::HashMap::new();
    if let Some(nation_snapshot) = snapshot.nations.get(&nation.entity()) {
        for civilian in &nation_snapshot.civilians {
            current_positions.insert(civilian.position, civilian.entity);
        }
    }

    // Sort civilian tasks topologically
    let execution_order =
        sort_civilian_tasks_topologically(&plan.civilian_tasks, &current_positions);

    // Send civilian orders in sorted order
    for (civilian_entity, task) in execution_order {
        if let Some(order) = task_to_order(&task) {
            civilian_commands.write(CivilianCommand {
                civilian: civilian_entity,
                order,
            });
        }
    }

    // Send market buy orders
    for (good, qty) in &plan.market_buys {
        commands.trigger(AdjustMarketOrder {
            nation,
            good: *good,
            kind: MarketInterest::Buy,
            requested: *qty,
        });
    }

    // Send market sell orders
    for (good, qty) in &plan.market_sells {
        commands.trigger(AdjustMarketOrder {
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
        commands.trigger(AdjustProduction {
            nation,
            building: order.building,
            output_good: order.output,
            target_output: order.qty,
        });
    }

    // Send transport allocation orders
    for (commodity, requested) in &plan.transport_allocations {
        commands.trigger(crate::economy::transport::TransportAdjustAllocation {
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

/// Sort tasks to resolve dependencies (A wants to move to B's tile -> B must move first).
fn sort_civilian_tasks_topologically(
    tasks: &std::collections::HashMap<Entity, CivilianTask>,
    current_positions: &std::collections::HashMap<bevy_ecs_tilemap::tiles::TilePos, Entity>,
) -> Vec<(Entity, CivilianTask)> {
    use std::collections::{HashMap, HashSet};

    let mut graph: HashMap<Entity, HashSet<Entity>> = HashMap::new();
    let mut in_degree: HashMap<Entity, usize> = HashMap::new();

    // Initialize graph nodes
    for &entity in tasks.keys() {
        graph.entry(entity).or_default();
        in_degree.entry(entity).or_insert(0);
    }

    // Build dependencies
    for (&actor, task) in tasks {
        if let CivilianTask::MoveTo { target } | CivilianTask::BuildRailTo { target } = task {
            // If target is occupied by another friendly unit
            if let Some(&occupier) = current_positions.get(target)
                && occupier != actor
                && tasks.contains_key(&occupier)
            {
                // actor depends on occupier moving
                // Edge: occupier -> actor (occupier must execute before actor)
                graph.entry(occupier).or_default().insert(actor);
                *in_degree.entry(actor).or_default() += 1;
            }
        }
    }

    // Kahn's Algorithm
    let mut queue: Vec<Entity> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&e, _)| e)
        .collect();

    // Sort queue to make behavior deterministic (e.g. by Entity ID)
    queue.sort();

    let mut sorted = Vec::new();

    while let Some(node) = queue.pop() {
        sorted.push(node);

        if let Some(neighbors) = graph.get(&node) {
            let mut neighbors_sorted: Vec<_> = neighbors.iter().copied().collect();
            neighbors_sorted.sort(); // Deterministic

            for neighbor in neighbors_sorted {
                let deg = in_degree.get_mut(&neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(neighbor);
                }
            }
        }
    }

    // Check for cycles (remaining nodes with in_degree > 0)
    let processed_count = sorted.len();
    if processed_count < tasks.len() {
        warn!("Cycle detected in AI movement commands! Breaking cycles arbitrarily.");
        // Add remaining nodes
        for &entity in tasks.keys() {
            if !sorted.contains(&entity) {
                sorted.push(entity);
            }
        }
    }

    // Map back to tasks
    sorted
        .into_iter()
        .filter_map(|e| tasks.get(&e).map(|t| (e, t.clone())))
        .collect()
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

    #[test]
    fn test_sort_civilian_tasks_topologically() {
        use std::collections::HashMap;

        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        let e3 = Entity::from_bits(3);

        let p1 = TilePos::new(0, 0);
        let p2 = TilePos::new(0, 1);
        let p3 = TilePos::new(0, 2);
        let p4 = TilePos::new(0, 3);

        // Chain: e1(at p1) -> p2, e2(at p2) -> p3, e3(at p3) -> p4
        let mut tasks = HashMap::new();
        tasks.insert(e1, CivilianTask::MoveTo { target: p2 });
        tasks.insert(e2, CivilianTask::MoveTo { target: p3 });
        tasks.insert(e3, CivilianTask::MoveTo { target: p4 });

        let mut positions = HashMap::new();
        positions.insert(p1, e1);
        positions.insert(p2, e2);
        positions.insert(p3, e3);

        let sorted = sort_civilian_tasks_topologically(&tasks, &positions);

        // Expected execution order: e3 (frees p3), then e2 (frees p2), then e1
        // Indices in result vector
        let idx1 = sorted.iter().position(|(e, _)| *e == e1).unwrap();
        let idx2 = sorted.iter().position(|(e, _)| *e == e2).unwrap();
        let idx3 = sorted.iter().position(|(e, _)| *e == e3).unwrap();

        assert!(idx3 < idx2, "e3 should execute before e2");
        assert!(idx2 < idx1, "e2 should execute before e1");
    }
}
