use bevy::prelude::*;
use std::collections::HashMap;

use super::goods::Good;
use super::reservation::ResourcePool;

/// Immutable view into a single stockpile entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockpileEntry {
    pub good: Good,
    pub total: u32,
    pub reserved: u32,
    pub available: u32,
}

#[derive(Component, Debug, Clone, Default)]
pub struct Stockpile {
    pools: HashMap<Good, ResourcePool>,
}

impl Stockpile {
    /// Get total amount of a good (including reserved)
    pub fn get(&self, good: Good) -> u32 {
        self.pools.get(&good).map(|p| p.total).unwrap_or(0)
    }

    /// Get reserved amount of a good
    pub fn get_reserved(&self, good: Good) -> u32 {
        self.pools.get(&good).map(|p| p.reserved).unwrap_or(0)
    }

    /// Get available amount of a good (total - reserved)
    pub fn get_available(&self, good: Good) -> u32 {
        self.pools.get(&good).map(|p| p.available()).unwrap_or(0)
    }

    /// Add resources to the stockpile
    pub fn add(&mut self, good: Good, qty: u32) {
        self.pools.entry(good).or_default().total += qty;
    }

    /// Reserve resources for a specific purpose (production, orders, etc.)
    /// Returns true if successful, false if not enough available
    pub fn reserve(&mut self, good: Good, qty: u32) -> bool {
        self.pools.entry(good).or_default().try_reserve(qty)
    }

    /// Unreserve resources (e.g., cancel an order)
    pub fn unreserve(&mut self, good: Good, qty: u32) {
        if let Some(pool) = self.pools.get_mut(&good) {
            pool.release(qty);
        }
    }

    /// Consume reserved resources (both from total and reserved)
    /// Should only be called after resources have been reserved
    pub fn consume_reserved(&mut self, good: Good, qty: u32) -> u32 {
        // Remove from reserved first
        let reserved = self.get_reserved(good);
        let to_consume = reserved.min(qty);
        self.unreserve(good, to_consume);

        // Then remove from total
        self.take_up_to(good, to_consume)
    }

    /// Attempts to remove `qty` units from total; returns how many were actually removed
    /// This is for immediate consumption (not going through reservation)
    pub fn take_up_to(&mut self, good: Good, qty: u32) -> u32 {
        let available = self.get(good);
        let take = available.min(qty);
        if take > 0
            && let Some(pool) = self.pools.get_mut(&good)
        {
            pool.total = pool.total.saturating_sub(take);
        }
        take
    }

    /// Returns true if the stockpile has at least `qty` units available (not reserved)
    pub fn has_available(&self, good: Good, qty: u32) -> bool {
        self.get_available(good) >= qty
    }

    /// Returns true if the stockpile has at least `qty` units total (including reserved)
    pub fn has_at_least(&self, good: Good, qty: u32) -> bool {
        self.get(good) >= qty
    }

    /// Iterate over all goods tracked by the stockpile.
    ///
    /// The iterator yields immutable snapshots containing total, reserved,
    /// and available quantities for each [`Good`]. Consumers should sort the
    /// returned collection if they require deterministic ordering.
    pub fn entries(&self) -> impl Iterator<Item = StockpileEntry> + '_ {
        self.pools.iter().map(|(good, pool)| StockpileEntry {
            good: *good,
            total: pool.total,
            reserved: pool.reserved,
            available: pool.available(),
        })
    }

    /// Internal: Get mutable access to a pool (for ReservationSystem)
    pub(super) fn get_pool_mut(&mut self, good: Good) -> Option<&mut ResourcePool> {
        Some(self.pools.entry(good).or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stockpile_add_and_take() {
        let mut s = Stockpile::default();
        assert_eq!(s.get(Good::Wool), 0);
        s.add(Good::Wool, 3);
        assert_eq!(s.get(Good::Wool), 3);
        assert!(s.has_at_least(Good::Wool, 2));
        let taken = s.take_up_to(Good::Wool, 5);
        assert_eq!(taken, 3);
        assert_eq!(s.get(Good::Wool), 0);
    }

    #[test]
    fn stockpile_reservation() {
        let mut s = Stockpile::default();
        s.add(Good::Wool, 10);

        // Reserve 3 units
        assert!(s.reserve(Good::Wool, 3));
        assert_eq!(s.get(Good::Wool), 10); // Total unchanged
        assert_eq!(s.get_reserved(Good::Wool), 3);
        assert_eq!(s.get_available(Good::Wool), 7); // 10 - 3

        // Try to reserve more than available
        assert!(!s.reserve(Good::Wool, 8)); // Only 7 available
        assert_eq!(s.get_reserved(Good::Wool), 3); // Unchanged

        // Reserve more (within available)
        assert!(s.reserve(Good::Wool, 5));
        assert_eq!(s.get_reserved(Good::Wool), 8); // 3 + 5
        assert_eq!(s.get_available(Good::Wool), 2); // 10 - 8

        // Unreserve some
        s.unreserve(Good::Wool, 3);
        assert_eq!(s.get_reserved(Good::Wool), 5);
        assert_eq!(s.get_available(Good::Wool), 5); // 10 - 5
    }

    #[test]
    fn stockpile_consume_reserved() {
        let mut s = Stockpile::default();
        s.add(Good::Wool, 10);
        s.reserve(Good::Wool, 5);

        // Consume reserved resources
        let consumed = s.consume_reserved(Good::Wool, 5);
        assert_eq!(consumed, 5);
        assert_eq!(s.get(Good::Wool), 5); // Total reduced
        assert_eq!(s.get_reserved(Good::Wool), 0); // Reservation cleared
        assert_eq!(s.get_available(Good::Wool), 5); // 5 - 0
    }

    #[test]
    fn stockpile_has_available() {
        let mut s = Stockpile::default();
        s.add(Good::Wool, 10);
        s.reserve(Good::Wool, 6);

        assert!(s.has_at_least(Good::Wool, 10)); // Total
        assert!(!s.has_at_least(Good::Wool, 11));
        assert!(s.has_available(Good::Wool, 4)); // Available (10 - 6)
        assert!(!s.has_available(Good::Wool, 5));
    }

    #[test]
    fn entries_expose_totals_and_reservations() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Grain, 5);
        stockpile.add(Good::Steel, 2);
        stockpile.reserve(Good::Grain, 3);

        let mut entries: Vec<_> = stockpile.entries().collect();
        entries.sort_by_key(|entry| entry.good);

        assert_eq!(entries.len(), 2);
        let grain = entries.iter().find(|e| e.good == Good::Grain).unwrap();
        assert_eq!(grain.total, 5);
        assert_eq!(grain.reserved, 3);
        assert_eq!(grain.available, 2);

        let steel = entries.iter().find(|e| e.good == Good::Steel).unwrap();
        assert_eq!(steel.total, 2);
        assert_eq!(steel.reserved, 0);
        assert_eq!(steel.available, 2);
    }
}
