use bevy::prelude::*;

use super::super::goods::Good;
use super::super::stockpile::Stockpile;
use super::systems::calculate_recruitment_cap;
use super::types::{RecruitmentCapacity, Workforce};
use crate::turn_system::{TurnPhase, TurnSystem};

/// Message to queue recruitment of untrained workers at the Capitol
#[derive(Message, Debug, Clone, Copy)]
pub struct RecruitWorkers {
    pub nation: Entity,
    pub count: u32,
}

/// Component tracking queued recruitment orders for a nation
#[derive(Component, Debug, Clone, Default)]
pub struct RecruitmentQueue {
    /// Number of workers queued for recruitment this turn
    pub queued: u32,
}

/// System to queue worker recruitment orders at the Capitol (Input Layer)
/// Validates resources exist and caps, reserves resources, queues the order
pub fn handle_recruitment(
    mut events: MessageReader<RecruitWorkers>,
    mut nations: Query<(&mut RecruitmentQueue, &mut Stockpile)>,
    recruitment_capacity: Query<&RecruitmentCapacity>,
    provinces: Query<&crate::province::Province>,
    mut log_writer: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for event in events.read() {
        if let Ok((mut queue, mut stockpile)) = nations.get_mut(event.nation) {
            // Calculate recruitment cap (count provinces owned by this nation entity)
            let province_count = provinces
                .iter()
                .filter(|p| p.owner == Some(event.nation))
                .count() as u32;

            let capacity = recruitment_capacity
                .get(event.nation)
                .map(|c| c.upgraded)
                .unwrap_or(false);

            let cap = calculate_recruitment_cap(province_count, capacity);

            // Limit requested count to cap
            let actual_count = event.count.min(cap);

            if actual_count == 0 {
                warn!("Cannot queue recruitment: cap is 0 (need more provinces)");
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: "Cannot recruit: need more provinces".to_string(),
                });
                continue;
            }

            // Check available resources (not already reserved/allocated)
            let canned_food_available = stockpile.get_available(Good::CannedFood);
            let clothing_available = stockpile.get_available(Good::Clothing);
            let furniture_available = stockpile.get_available(Good::Furniture);

            // How many can we actually recruit with available resources?
            let max_by_resources = canned_food_available
                .min(clothing_available)
                .min(furniture_available);

            let final_count = actual_count.min(max_by_resources);

            if final_count == 0 {
                warn!(
                    "Cannot queue recruitment: not enough available resources (need: {} each, available: {} food, {} clothing, {} furniture)",
                    actual_count, canned_food_available, clothing_available, furniture_available
                );
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "Cannot recruit: need Canned Food, Clothing, Furniture (available: {}, {}, {})",
                        canned_food_available, clothing_available, furniture_available
                    ),
                });
                continue;
            }

            // Reserve the resources
            if !stockpile.reserve(Good::CannedFood, final_count)
                || !stockpile.reserve(Good::Clothing, final_count)
                || !stockpile.reserve(Good::Furniture, final_count)
            {
                warn!("Failed to reserve resources (race condition?)");
                continue;
            }

            // Queue the order (resources now reserved)
            queue.queued += final_count;

            info!(
                "Queued {} workers for recruitment (total queued: {}, cap: {})",
                final_count, queue.queued, cap
            );
            log_writer.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "Queued {} workers for recruitment (will hire next turn)",
                    final_count
                ),
            });
        }
    }
}

/// System to execute queued recruitment orders during turn processing (Logic Layer)
pub fn execute_recruitment_orders(
    turn: Res<TurnSystem>,
    mut nations: Query<(&mut RecruitmentQueue, &mut Workforce, &mut Stockpile)>,
) {
    // Only execute during Processing phase
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (mut queue, mut workforce, mut stockpile) in nations.iter_mut() {
        if queue.queued == 0 {
            continue;
        }

        let count = queue.queued;

        // Consume reserved resources (both from reserved and total)
        stockpile.consume_reserved(Good::CannedFood, count);
        stockpile.consume_reserved(Good::Clothing, count);
        stockpile.consume_reserved(Good::Furniture, count);

        // Add workers
        workforce.add_untrained(count);

        info!("Recruited {} untrained workers", count);

        // Clear the queue
        queue.queued = 0;
    }
}
