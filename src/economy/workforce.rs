use bevy::prelude::*;

use super::goods::Good;
use super::stockpile::Stockpile;
use crate::turn_system::{TurnPhase, TurnSystem};

/// Message to queue recruitment of untrained workers at the Capitol
#[derive(Message, Debug, Clone, Copy)]
pub struct RecruitWorkers {
    pub nation: Entity,
    pub count: u32,
}

/// Message to queue training of a worker at the Trade School
#[derive(Message, Debug, Clone, Copy)]
pub struct TrainWorker {
    pub nation: Entity,
    pub from_skill: WorkerSkill,
}

/// Component tracking queued recruitment orders for a nation
#[derive(Component, Debug, Clone, Default)]
pub struct RecruitmentQueue {
    /// Number of workers queued for recruitment this turn
    pub queued: u32,
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

/// Workforce component tracks workers by skill level for a nation
/// Workers provide labor points: Untrained=1, Trained=2, Expert=4
#[derive(Component, Debug, Clone, Default)]
pub struct Workforce {
    /// Individual workers with their state
    pub workers: Vec<Worker>,
}

impl Workforce {
    /// Creates an empty workforce
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
        }
    }

    /// Adds untrained workers (for migration)
    pub fn add_untrained(&mut self, count: u32) {
        for _ in 0..count {
            self.workers.push(Worker {
                skill: WorkerSkill::Untrained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }
    }

    /// Count workers by skill level
    pub fn count_by_skill(&self, skill: WorkerSkill) -> u32 {
        self.workers.iter().filter(|w| w.skill == skill).count() as u32
    }

    /// Count untrained workers
    pub fn untrained_count(&self) -> u32 {
        self.count_by_skill(WorkerSkill::Untrained)
    }

    /// Count trained workers
    pub fn trained_count(&self) -> u32 {
        self.count_by_skill(WorkerSkill::Trained)
    }

    /// Count expert workers
    pub fn expert_count(&self) -> u32 {
        self.count_by_skill(WorkerSkill::Expert)
    }

    /// Calculate total available labor points from healthy workers
    pub fn available_labor(&self) -> u32 {
        self.workers
            .iter()
            .filter(|w| w.health == WorkerHealth::Healthy)
            .map(|w| w.skill.labor_points())
            .sum()
    }

    /// Train a worker from Untrained to Trained or Trained to Expert
    /// Returns true if a worker was trained, false if none available
    pub fn train_worker(&mut self, from_skill: WorkerSkill) -> bool {
        if let Some(worker) = self
            .workers
            .iter_mut()
            .find(|w| w.skill == from_skill && w.health == WorkerHealth::Healthy)
        {
            worker.skill = from_skill.next_level();
            true
        } else {
            false
        }
    }

    /// Remove an expert worker (e.g., for building a civilian unit)
    /// Returns true if a worker was removed, false if none available
    pub fn consume_expert(&mut self) -> bool {
        if let Some(idx) = self
            .workers
            .iter()
            .position(|w| w.skill == WorkerSkill::Expert && w.health == WorkerHealth::Healthy)
        {
            self.workers.remove(idx);
            true
        } else {
            false
        }
    }

    /// Reset all workers to healthy at start of turn
    pub fn reset_health(&mut self) {
        for worker in self.workers.iter_mut() {
            worker.health = WorkerHealth::Healthy;
        }
    }

    /// Assign food preferences to workers (cyclic pattern: Grain → Fruit → Livestock/Fish)
    pub fn assign_food_preferences(&mut self) {
        for (i, worker) in self.workers.iter_mut().enumerate() {
            worker.food_preference_slot = (i % 3) as u8;
        }
    }

    /// Get the preferred food for a worker's slot
    pub fn preferred_food_for_slot(slot: u8) -> Good {
        match slot % 3 {
            0 => Good::Grain,
            1 => Good::Fruit,
            2 => Good::Livestock, // or Fish, but we'll use Livestock as default
            _ => unreachable!(),
        }
    }

    /// Remove dead workers
    pub fn remove_dead(&mut self) {
        self.workers.retain(|w| w.health != WorkerHealth::Dead);
    }
}

/// Individual worker with skill level and health state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worker {
    pub skill: WorkerSkill,
    pub health: WorkerHealth,
    /// Food preference slot (0=Grain, 1=Fruit, 2=Livestock/Fish)
    pub food_preference_slot: u8,
}

/// Worker skill level determines labor points
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerSkill {
    Untrained, // 1 labor point
    Trained,   // 2 labor points
    Expert,    // 4 labor points
}

impl WorkerSkill {
    /// Labor points provided by this skill level
    pub fn labor_points(self) -> u32 {
        match self {
            WorkerSkill::Untrained => 1,
            WorkerSkill::Trained => 2,
            WorkerSkill::Expert => 4,
        }
    }

    /// Next skill level (for training)
    pub fn next_level(self) -> Self {
        match self {
            WorkerSkill::Untrained => WorkerSkill::Trained,
            WorkerSkill::Trained => WorkerSkill::Expert,
            WorkerSkill::Expert => WorkerSkill::Expert, // Already max
        }
    }
}

/// Worker health state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerHealth {
    Healthy, // Produces labor
    Sick,    // Ate wrong food, produces 0 labor
    Dead,    // No food at all, will be removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workforce_creation() {
        let mut workforce = Workforce::new();
        assert_eq!(workforce.untrained_count(), 0);
        assert_eq!(workforce.trained_count(), 0);
        assert_eq!(workforce.expert_count(), 0);
        assert_eq!(workforce.available_labor(), 0);

        workforce.add_untrained(5);
        assert_eq!(workforce.untrained_count(), 5);
        assert_eq!(workforce.available_labor(), 5); // 5 × 1 = 5
    }

    #[test]
    fn labor_calculation() {
        let mut workforce = Workforce::new();
        workforce.add_untrained(2);
        workforce.workers.push(Worker {
            skill: WorkerSkill::Trained,
            health: WorkerHealth::Healthy,
            food_preference_slot: 0,
        });
        workforce.workers.push(Worker {
            skill: WorkerSkill::Expert,
            health: WorkerHealth::Healthy,
            food_preference_slot: 0,
        });

        // 2 untrained (2×1) + 1 trained (1×2) + 1 expert (1×4) = 8
        assert_eq!(workforce.available_labor(), 8);
    }

    #[test]
    fn sick_workers_no_labor() {
        let mut workforce = Workforce::new();
        workforce.workers.push(Worker {
            skill: WorkerSkill::Expert,
            health: WorkerHealth::Sick,
            food_preference_slot: 0,
        });
        assert_eq!(workforce.available_labor(), 0);
    }

    #[test]
    fn training() {
        let mut workforce = Workforce::new();
        workforce.add_untrained(2);

        assert!(workforce.train_worker(WorkerSkill::Untrained));
        assert_eq!(workforce.untrained_count(), 1);
        assert_eq!(workforce.trained_count(), 1);

        assert!(workforce.train_worker(WorkerSkill::Trained));
        assert_eq!(workforce.trained_count(), 0);
        assert_eq!(workforce.expert_count(), 1);
    }

    #[test]
    fn consume_expert() {
        let mut workforce = Workforce::new();
        workforce.workers.push(Worker {
            skill: WorkerSkill::Expert,
            health: WorkerHealth::Healthy,
            food_preference_slot: 0,
        });

        assert_eq!(workforce.expert_count(), 1);
        assert!(workforce.consume_expert());
        assert_eq!(workforce.expert_count(), 0);
        assert!(!workforce.consume_expert()); // No more experts
    }

    #[test]
    fn food_preferences() {
        let mut workforce = Workforce::new();
        workforce.add_untrained(4);
        workforce.assign_food_preferences();

        assert_eq!(workforce.workers[0].food_preference_slot, 0); // Grain
        assert_eq!(workforce.workers[1].food_preference_slot, 1); // Fruit
        assert_eq!(workforce.workers[2].food_preference_slot, 2); // Livestock
        assert_eq!(workforce.workers[3].food_preference_slot, 0); // Grain again
    }

    #[test]
    fn remove_dead() {
        let mut workforce = Workforce::new();
        workforce.workers.push(Worker {
            skill: WorkerSkill::Untrained,
            health: WorkerHealth::Dead,
            food_preference_slot: 0,
        });
        workforce.workers.push(Worker {
            skill: WorkerSkill::Trained,
            health: WorkerHealth::Healthy,
            food_preference_slot: 1,
        });

        assert_eq!(workforce.workers.len(), 2);
        workforce.remove_dead();
        assert_eq!(workforce.workers.len(), 1);
        assert_eq!(workforce.workers[0].skill, WorkerSkill::Trained);
    }
}

/// System that feeds workers at the start of each player turn
/// Implements the feeding preference cycle: preferred raw → canned → wrong raw (sick) → none (dead)
pub fn feed_workers(
    turn: Res<TurnSystem>,
    mut nations: Query<(Entity, &mut Workforce, &mut Stockpile)>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    mut log_writer: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    // Only run at start of player turn
    if turn.phase != TurnPhase::PlayerTurn {
        return;
    }

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
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "WARNING: {} workers sick (ate wrong food, 0 labor)",
                        sick_count
                    ),
                });
            }
            if dead_count > 0 {
                warn!("{} workers died from starvation", dead_count);
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!("ALERT: {} workers died from starvation!", dead_count),
                });
            }
        }

        // Remove dead workers
        workforce.remove_dead();
    }
}

/// Component to track recruitment upgrades
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RecruitmentCapacity {
    pub upgraded: bool, // false = provinces/4, true = provinces/3
}

/// Calculate recruitment cap based on province count
pub fn calculate_recruitment_cap(province_count: u32, upgraded: bool) -> u32 {
    if upgraded {
        province_count / 3
    } else {
        province_count / 4
    }
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
            if treasury.0 < total_cash_needed {
                warn!("Cannot queue training: not enough money");
                log_writer.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "Cannot train: need ${} (have: ${})",
                        total_cash_needed, treasury.0
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
                    treasury.0 -= TRAINING_COST_CASH;

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
