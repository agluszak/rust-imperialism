use bevy::prelude::*;

use super::super::goods::Good;
use super::super::stockpile::Stockpile;
use super::types::{WorkerSkill, Workforce};
use crate::turn_system::{TurnPhase, TurnSystem};

/// Message to queue training of a worker at the Trade School
#[derive(Message, Debug, Clone, Copy)]
pub struct TrainWorker {
    pub nation: Entity,
    pub from_skill: WorkerSkill,
}

/// Component tracking queued training orders for a nation
#[derive(Component, Debug, Clone, Default)]
pub struct TrainingQueue {
    /// Training orders: (from_skill, count)
    pub orders: Vec<(WorkerSkill, u32)>,
}

impl TrainingQueue {
    pub fn add_order(&mut self, from_skill: WorkerSkill, count: u32) {
        // Find existing order for this skill level or add new one
        if let Some(order) = self
            .orders
            .iter_mut()
            .find(|(skill, _)| *skill == from_skill)
        {
            order.1 += count;
        } else {
            self.orders.push((from_skill, count));
        }
    }

    pub fn total_queued(&self) -> u32 {
        self.orders.iter().map(|(_, count)| count).sum()
    }
}

/// System to queue worker training orders at the Trade School (Input Layer)
/// Validates resources exist, reserves them, queues the order
pub fn handle_training(
    mut events: MessageReader<TrainWorker>,
    mut nations: Query<(
        &Workforce,
        &mut Stockpile,
        &crate::economy::treasury::Treasury,
        &mut TrainingQueue,
    )>,
    mut log_writer: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    const TRAINING_COST_PAPER: u32 = 1;
    const TRAINING_COST_CASH: i64 = 100;

    for event in events.read() {
        if let Ok((workforce, mut stockpile, treasury, mut queue)) = nations.get_mut(event.nation) {
            // Calculate total training orders of this type (including queued)
            let already_queued = queue
                .orders
                .iter()
                .find(|(skill, _)| *skill == event.from_skill)
                .map(|(_, count)| *count)
                .unwrap_or(0);

            let total_orders = already_queued + 1;

            // Check if we have enough workers to train
            let available = workforce.count_by_skill(event.from_skill);
            if available < total_orders {
                let skill_name = match event.from_skill {
                    WorkerSkill::Untrained => "Untrained",
                    WorkerSkill::Trained => "Trained",
                    WorkerSkill::Expert => "Expert",
                };
                warn!("Cannot queue training: not enough {} workers", skill_name);
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "Cannot train: not enough {} workers (have: {}, need: {})",
                        skill_name, available, total_orders
                    ),
                });
                continue;
            }

            // Check if we have the required available resources (not reserved)
            if !stockpile.has_available(Good::Paper, TRAINING_COST_PAPER) {
                warn!("Cannot queue training: not enough available paper");
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "Cannot train: need {} Paper (available: {})",
                        TRAINING_COST_PAPER,
                        stockpile.get_available(Good::Paper)
                    ),
                });
                continue;
            }

            // TODO: Treasury reservations would need a separate system
            // For now, just check total cash
            let total_queued = queue.total_queued();
            let total_cash_needed = (total_queued as i64 + 1) * TRAINING_COST_CASH;
            if treasury.total() < total_cash_needed {
                warn!("Cannot queue training: not enough money");
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "Cannot train: need ${} (have: ${})",
                        total_cash_needed,
                        treasury.total()
                    ),
                });
                continue;
            }

            // Reserve the resources
            if !stockpile.reserve(Good::Paper, TRAINING_COST_PAPER) {
                warn!("Failed to reserve paper (race condition?)");
                continue;
            }

            // Queue the order
            queue.add_order(event.from_skill, 1);

            let from_name = match event.from_skill {
                WorkerSkill::Untrained => "Untrained",
                WorkerSkill::Trained => "Trained",
                WorkerSkill::Expert => "Expert",
            };
            let to_name = match event.from_skill.next_level() {
                WorkerSkill::Untrained => "Untrained",
                WorkerSkill::Trained => "Trained",
                WorkerSkill::Expert => "Expert",
            };

            info!("Queued training: {} -> {}", from_name, to_name);
            log_writer.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "Queued training: {} -> {} (will train next turn)",
                    from_name, to_name
                ),
            });
        }
    }
}

/// System to execute queued training orders during turn processing (Logic Layer)
pub fn execute_training_orders(
    turn: Res<TurnSystem>,
    mut nations: Query<(
        &mut TrainingQueue,
        &mut Workforce,
        &mut Stockpile,
        &mut crate::economy::treasury::Treasury,
    )>,
) {
    const TRAINING_COST_PAPER: u32 = 1;
    const TRAINING_COST_CASH: i64 = 100;

    // Only execute during Processing phase
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (mut queue, mut workforce, mut stockpile, mut treasury) in nations.iter_mut() {
        if queue.orders.is_empty() {
            continue;
        }

        // Process all training orders
        for (from_skill, count) in queue.orders.iter() {
            for _ in 0..*count {
                // Train the worker
                if workforce.train_worker(*from_skill) {
                    // Consume reserved resources
                    stockpile.consume_reserved(Good::Paper, TRAINING_COST_PAPER);
                    treasury.subtract(TRAINING_COST_CASH);

                    info!(
                        "Trained worker from {:?} to {:?}",
                        from_skill,
                        from_skill.next_level()
                    );
                }
            }
        }

        // Clear the queue
        queue.orders.clear();
    }
}
