use bevy::prelude::*;

use crate::economy::goods::Good;
use crate::economy::reservation::ResourcePool;

/// Workforce component tracks workers by skill level for a nation
/// Workers provide labor points: Untrained=1, Trained=2, Expert=4
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct Workforce {
    /// Individual workers with their state
    pub workers: Vec<Worker>,
    /// Labor pool for reservations
    pub labor_pool: ResourcePool,
}

impl Workforce {
    /// Creates an empty workforce
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
            labor_pool: ResourcePool::default(),
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

    /// Update labor pool total based on current worker state
    /// Should be called at start of turn after health resets
    pub fn update_labor_pool(&mut self) {
        self.labor_pool.total = self.available_labor();
    }

    /// Try to reserve labor (for ReservationSystem)
    pub fn try_reserve_labor(&mut self, amount: u32) -> bool {
        self.labor_pool.try_reserve(amount)
    }

    /// Release labor reservation (for ReservationSystem)
    pub fn release_labor(&mut self, amount: u32) {
        self.labor_pool.release(amount);
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

    /// Assign food preferences to workers (cyclic pattern: Grain → Grain → Fruit → Livestock/Fish)
    /// Distribution: 50% Grain, 25% Fruit, 25% Meat (Livestock/Fish)
    pub fn assign_food_preferences(&mut self) {
        for (i, worker) in self.workers.iter_mut().enumerate() {
            worker.food_preference_slot = (i % 4) as u8;
        }
    }

    /// Get the preferred food for a worker's slot
    pub fn preferred_food_for_slot(slot: u8) -> Good {
        match slot % 4 {
            0 | 1 => Good::Grain,
            2 => Good::Fruit,
            3 => Good::Livestock, // or Fish, checked in consumption logic
            _ => unreachable!(),
        }
    }

    /// Remove dead workers
    pub fn remove_dead(&mut self) {
        self.workers.retain(|w| w.health != WorkerHealth::Dead);
    }
}

/// Individual worker with skill level and health state
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct Worker {
    pub skill: WorkerSkill,
    pub health: WorkerHealth,
    /// Food preference slot (0=Grain, 1=Fruit, 2=Livestock/Fish)
    pub food_preference_slot: u8,
}

/// Worker skill level determines labor points
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum WorkerHealth {
    Healthy, // Produces labor
    Sick,    // Ate wrong food, produces 0 labor
    Dead,    // No food at all, will be removed
}

/// Component to track recruitment upgrades
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct RecruitmentCapacity {
    pub upgraded: bool, // false = provinces/4, true = provinces/3
}

#[cfg(test)]
mod tests {
    use crate::economy::workforce::*;

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
        assert_eq!(workforce.workers[1].food_preference_slot, 1); // Grain
        assert_eq!(workforce.workers[2].food_preference_slot, 2); // Fruit
        assert_eq!(workforce.workers[3].food_preference_slot, 3); // Meat
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
