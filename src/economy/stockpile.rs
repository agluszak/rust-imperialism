use bevy::prelude::*;
use std::collections::HashMap;

use super::goods::Good;

#[derive(Component, Debug, Clone, Default)]
pub struct Stockpile {
    /// Total resources in the stockpile
    pub total: HashMap<Good, u32>,
    /// Resources committed/reserved for various purposes (production, orders, etc.)
    pub reserved: HashMap<Good, u32>,
}

impl Stockpile {
    /// Get total amount of a good (including reserved)
    pub fn get(&self, good: Good) -> u32 {
        *self.total.get(&good).unwrap_or(&0)
    }

    /// Get reserved amount of a good
    pub fn get_reserved(&self, good: Good) -> u32 {
        *self.reserved.get(&good).unwrap_or(&0)
    }

    /// Get available amount of a good (total - reserved)
    pub fn get_available(&self, good: Good) -> u32 {
        self.get(good).saturating_sub(self.get_reserved(good))
    }

    /// Add resources to the stockpile
    pub fn add(&mut self, good: Good, qty: u32) {
        *self.total.entry(good).or_default() += qty;
    }

    /// Reserve resources for a specific purpose (production, orders, etc.)
    /// Returns true if successful, false if not enough available
    pub fn reserve(&mut self, good: Good, qty: u32) -> bool {
        if self.get_available(good) >= qty {
            *self.reserved.entry(good).or_default() += qty;
            true
        } else {
            false
        }
    }

    /// Unreserve resources (e.g., cancel an order)
    pub fn unreserve(&mut self, good: Good, qty: u32) {
        let current = self.get_reserved(good);
        let new_reserved = current.saturating_sub(qty);
        if new_reserved == 0 {
            self.reserved.remove(&good);
        } else {
            self.reserved.insert(good, new_reserved);
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
        if take > 0 {
            self.total.insert(good, available - take);
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
}
