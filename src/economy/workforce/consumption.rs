use bevy::prelude::*;

use crate::economy::PlayerNation;
use crate::economy::goods::Good;
use crate::economy::stockpile::Stockpile;
use crate::economy::workforce::types::{WorkerHealth, Workforce};

/// System that feeds workers at the start of each player turn
/// Implements the feeding preference cycle: preferred raw → canned → wrong raw (sick) → none (dead)
/// NOTE: Registered via OnEnter(TurnPhase::PlayerTurn), so no phase check needed.
pub fn feed_workers(
    mut nations: Query<(Entity, &mut Workforce, &mut Stockpile)>,
    player_nation: Option<Res<PlayerNation>>,
) {
    for (entity, mut workforce, mut stockpile) in nations.iter_mut() {
        let is_player = player_nation
            .as_ref()
            .map(|p| p.0 == entity)
            .unwrap_or(false);

        // Assign food preferences (cyclic pattern)
        workforce.assign_food_preferences();

        let mut sick_count = 0;
        let mut dead_count = 0;

        // Feed each worker
        for worker in workforce.workers.iter_mut() {
            let preferred_food = Workforce::preferred_food_for_slot(worker.food_preference_slot);

            // Try preferred raw food first
            if stockpile.has_at_least(preferred_food, 1) {
                stockpile.take_up_to(preferred_food, 1);
                worker.health = WorkerHealth::Healthy;
            }
            // Try canned food as fallback
            else if stockpile.has_at_least(Good::CannedFood, 1) {
                stockpile.take_up_to(Good::CannedFood, 1);
                worker.health = WorkerHealth::Healthy;
            }
            // Try any other raw food (makes worker sick)
            else if let Some(alt_food) = [Good::Grain, Good::Fruit, Good::Livestock, Good::Fish]
                .iter()
                .find(|&&food| food != preferred_food && stockpile.has_at_least(food, 1))
            {
                stockpile.take_up_to(*alt_food, 1);
                worker.health = WorkerHealth::Sick;
                sick_count += 1;
            }
            // No food at all (worker dies)
            else {
                worker.health = WorkerHealth::Dead;
                dead_count += 1;
            }
        }

        // Log warnings for player only
        if is_player {
            if sick_count > 0 {
                warn!("{} workers got sick from wrong food", sick_count);
                info!(
                    "WARNING: {} workers sick (ate wrong food, 0 labor)",
                    sick_count
                );
            }
            if dead_count > 0 {
                warn!("{} workers died from starvation", dead_count);
                info!("ALERT: {} workers died from starvation!", dead_count);
            }
        }

        // Remove dead workers
        workforce.remove_dead();
    }
}
