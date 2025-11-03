use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::goods::Good;

/// A pool of resources with reservations
#[derive(Debug, Clone, Default)]
pub struct ResourcePool {
    pub total: u32,
    pub reserved: u32,
}

impl ResourcePool {
    pub fn new(total: u32) -> Self {
        Self { total, reserved: 0 }
    }

    /// Get available (unreserved) amount
    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.reserved)
    }

    /// Try to reserve amount - returns true if successful
    /// Returns false if insufficient resources available
    pub fn try_reserve(&mut self, amount: u32) -> bool {
        if amount <= self.available() {
            self.reserved += amount;
            true
        } else {
            false
        }
    }

    /// Release a reservation
    pub fn release(&mut self, amount: u32) {
        self.reserved = self.reserved.saturating_sub(amount);
    }

    /// Consume all reservations (turn resources into actual usage)
    pub fn consume_reserved(&mut self) {
        self.total = self.total.saturating_sub(self.reserved);
        self.reserved = 0;
    }
}

/// Opaque identifier for a reservation
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ReservationId(u32);

/// Internal data for a reservation
#[derive(Debug, Clone)]
struct ReservationData {
    goods: Vec<(Good, u32)>,
    labor: u32,
    money: u32,
}

/// Per-nation reservation tracking system
/// Each nation has its own instance as a Component
#[derive(Component, Debug, Default)]
pub struct ReservationSystem {
    next_id: u32,
    reservations: HashMap<ReservationId, ReservationData>,
}

impl ReservationSystem {
    /// Try to reserve multiple resources atomically (all-or-nothing)
    /// Returns ReservationId on success, None if any resource unavailable
    pub fn try_reserve(
        &mut self,
        goods: Vec<(Good, u32)>,
        labor: u32,
        money: u32,
        stockpile: &mut crate::economy::stockpile::Stockpile,
        workforce: &mut crate::economy::workforce::Workforce,
        treasury: &mut crate::economy::treasury::Treasury,
    ) -> Option<ReservationId> {
        let mut reserved_goods = Vec::new();

        // Try to reserve all goods
        for (good, amount) in &goods {
            if let Some(pool) = stockpile.get_pool_mut(*good) {
                let available = pool.available();
                if pool.try_reserve(*amount) {
                    reserved_goods.push((*good, *amount));
                } else {
                    // ROLLBACK: release everything we reserved so far
                    for (g, amt) in reserved_goods {
                        if let Some(pool) = stockpile.get_pool_mut(g) {
                            pool.release(amt);
                        }
                    }
                    info!(
                        "Reservation failed: insufficient {:?} (need {}, have {})",
                        good, amount, available
                    );
                    return None;
                }
            } else {
                // Good doesn't exist in stockpile - fail and rollback
                for (g, amt) in reserved_goods {
                    if let Some(pool) = stockpile.get_pool_mut(g) {
                        pool.release(amt);
                    }
                }
                info!("Reservation failed: {:?} not in stockpile", good);
                return None;
            }
        }

        // Try to reserve labor
        let labor_available = workforce.labor_pool.available();
        if !workforce.try_reserve_labor(labor) {
            // ROLLBACK: release goods
            for (good, amt) in reserved_goods {
                if let Some(pool) = stockpile.get_pool_mut(good) {
                    pool.release(amt);
                }
            }
            info!(
                "Reservation failed: insufficient labor (need {}, have {})",
                labor, labor_available
            );
            return None;
        }

        // Try to reserve money
        let money_available = treasury.available();
        if !treasury.try_reserve(money) {
            // ROLLBACK: release goods and labor
            for (good, amt) in reserved_goods {
                if let Some(pool) = stockpile.get_pool_mut(good) {
                    pool.release(amt);
                }
            }
            workforce.release_labor(labor);
            info!(
                "Reservation failed: insufficient money (need {}, have {})",
                money, money_available
            );
            return None;
        }

        // SUCCESS - store in database
        let id = ReservationId(self.next_id);
        self.next_id += 1;
        self.reservations.insert(
            id,
            ReservationData {
                goods,
                labor,
                money,
            },
        );
        Some(id)
    }

    /// Release a reservation (puts resources back, consumes the reservation)
    pub fn release(
        &mut self,
        id: ReservationId,
        stockpile: &mut crate::economy::stockpile::Stockpile,
        workforce: &mut crate::economy::workforce::Workforce,
        treasury: &mut crate::economy::treasury::Treasury,
    ) {
        if let Some(data) = self.reservations.remove(&id) {
            for (good, amt) in data.goods {
                if let Some(pool) = stockpile.get_pool_mut(good) {
                    pool.release(amt);
                }
            }
            workforce.release_labor(data.labor);
            treasury.release(data.money);
        }
    }

    /// Consume a single reservation (commits it and removes from database)
    /// The resources are consumed (total reduced, reserved cleared)
    pub fn consume(
        &mut self,
        id: ReservationId,
        stockpile: &mut crate::economy::stockpile::Stockpile,
        workforce: &mut crate::economy::workforce::Workforce,
        treasury: &mut crate::economy::treasury::Treasury,
    ) {
        if let Some(data) = self.reservations.remove(&id) {
            // For each reserved resource, consume it (subtract from total, clear reservation)
            for (good, _amt) in data.goods {
                if let Some(pool) = stockpile.get_pool_mut(good) {
                    pool.consume_reserved();
                }
            }
            workforce.labor_pool.consume_reserved();
            treasury.consume_reserved();
        }
    }

    /// Consume all reservations (at turn end - commits them)
    /// The actual resource consumption happens in the pools themselves
    pub fn consume_all(&mut self) {
        self.reservations.clear();
    }

    /// Get count of active reservations (for debugging/UI)
    pub fn count(&self) -> usize {
        self.reservations.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::economy::*;

    #[test]
    fn resource_pool_basic() {
        let mut pool = ResourcePool::new(10);
        assert_eq!(pool.available(), 10);

        assert!(pool.try_reserve(5));
        assert_eq!(pool.available(), 5);
        assert_eq!(pool.reserved, 5);

        assert!(pool.try_reserve(5));
        assert_eq!(pool.available(), 0);

        assert!(!pool.try_reserve(1)); // Should fail
    }

    #[test]
    fn resource_pool_release() {
        let mut pool = ResourcePool::new(10);
        pool.try_reserve(7);
        assert_eq!(pool.available(), 3);

        pool.release(3);
        assert_eq!(pool.available(), 6);
        assert_eq!(pool.reserved, 4);
    }

    #[test]
    fn resource_pool_consume() {
        let mut pool = ResourcePool::new(10);
        pool.try_reserve(4);

        pool.consume_reserved();

        assert_eq!(pool.total, 6);
        assert_eq!(pool.reserved, 0);
        assert_eq!(pool.available(), 6);
    }
}
