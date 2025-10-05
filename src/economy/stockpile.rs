use bevy::prelude::*;
use std::collections::HashMap;

use super::goods::Good;

#[derive(Component, Debug, Default, Clone)]
pub struct Stockpile(pub HashMap<Good, u32>);

impl Stockpile {
    pub fn get(&self, good: Good) -> u32 {
        *self.0.get(&good).unwrap_or(&0)
    }

    pub fn add(&mut self, good: Good, qty: u32) {
        *self.0.entry(good).or_default() += qty;
    }

    /// Attempts to remove `qty` units; returns how many were actually removed
    pub fn take_up_to(&mut self, good: Good, qty: u32) -> u32 {
        let available = self.get(good);
        let take = available.min(qty);
        if take > 0 {
            self.0.insert(good, available - take);
        }
        take
    }

    /// Returns true if the stockpile has at least `qty` units
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
}
