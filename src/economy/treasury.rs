use bevy::prelude::*;

use crate::economy::reservation::ResourcePool;

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Treasury {
    money_pool: ResourcePool,
}

impl Default for Treasury {
    fn default() -> Self {
        Treasury {
            money_pool: ResourcePool::new(50_000),
        }
    }
}

impl Treasury {
    pub fn new(amount: u32) -> Self {
        Treasury {
            money_pool: ResourcePool::new(amount),
        }
    }

    /// Get total money (including reserved)
    pub fn total(&self) -> i64 {
        self.money_pool.total as i64
    }

    /// Get reserved money
    pub fn reserved(&self) -> i64 {
        self.money_pool.reserved as i64
    }

    /// Get available money (total - reserved)
    pub fn available(&self) -> i64 {
        self.money_pool.available() as i64
    }

    /// Add money
    pub fn add(&mut self, amount: i64) {
        if amount > 0 {
            self.money_pool.total = self.money_pool.total.saturating_add(amount as u32);
        }
    }

    /// Subtract money (immediate, not through reservation)
    pub fn subtract(&mut self, amount: i64) {
        if amount > 0 {
            self.money_pool.total = self.money_pool.total.saturating_sub(amount as u32);
        }
    }

    /// Try to reserve money (for ReservationSystem)
    pub fn try_reserve(&mut self, amount: u32) -> bool {
        self.money_pool.try_reserve(amount)
    }

    /// Release money reservation (for ReservationSystem)
    pub fn release(&mut self, amount: u32) {
        self.money_pool.release(amount);
    }

    /// Consume reserved money (for ReservationSystem)
    pub fn consume_reserved(&mut self) {
        self.money_pool.consume_reserved();
    }
}

// Compatibility: allow tuple-like access for existing code
impl From<i64> for Treasury {
    fn from(amount: i64) -> Self {
        Treasury::new(amount.max(0) as u32)
    }
}
