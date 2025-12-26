use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::nation::Nation;
use crate::ships::Ship;

/// Base amount of cargo holds every nation starts with.
pub const BASE_TRADE_CAPACITY: u32 = 3;

/// Tracks how many cargo holds (merchant ships) each nation controls.
#[derive(Default, Resource, Debug, Clone)]
pub struct TradeCapacity {
    pub nations: HashMap<Entity, TradeCapacitySnapshot>,
}

#[derive(Debug, Clone, Copy)]
pub struct TradeCapacitySnapshot {
    pub total: u32,
    pub used: u32,
}

impl Default for TradeCapacitySnapshot {
    fn default() -> Self {
        Self {
            total: BASE_TRADE_CAPACITY,
            used: 0,
        }
    }
}

impl TradeCapacitySnapshot {
    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.used)
    }
}

impl TradeCapacity {
    pub fn snapshot(&self, nation: Entity) -> TradeCapacitySnapshot {
        self.nations.get(&nation).copied().unwrap_or_default()
    }

    pub fn snapshot_mut(&mut self, nation: Entity) -> &mut TradeCapacitySnapshot {
        self.nations.entry(nation).or_default()
    }

    pub fn available(&self, nation: Entity) -> u32 {
        self.snapshot(nation).available()
    }

    pub fn consume(&mut self, nation: Entity, amount: u32) -> bool {
        let snapshot = self.snapshot_mut(nation);
        if snapshot.used + amount > snapshot.total {
            return false;
        }
        snapshot.used += amount;
        true
    }

    /// Reset usage counters for the next round of trading.
    pub fn reset_usage(&mut self) {
        for snapshot in self.nations.values_mut() {
            snapshot.used = 0;
        }
    }
}

/// Ensure newly created nations start with baseline trade capacity.
pub fn initialize_trade_capacity(
    mut capacity: ResMut<TradeCapacity>,
    nations: Query<Entity, Added<Nation>>,
) {
    for nation in nations.iter() {
        capacity.snapshot_mut(nation);
    }
}

/// Update trade capacity based on ships owned by each nation.
/// Runs at the start of PlayerTurn phase.
pub fn update_trade_capacity_from_ships(
    mut capacity: ResMut<TradeCapacity>,
    ships: Query<&Ship>,
    nations: Query<Entity, With<Nation>>,
) {
    for nation in nations.iter() {
        // Count total cargo capacity from all ships owned by this nation
        let total_from_ships: u32 = ships
            .iter()
            .filter(|ship| ship.owner == nation)
            .map(|ship| ship.cargo_capacity())
            .sum();

        // Update the nation's trade capacity (base + ships)
        let snapshot = capacity.snapshot_mut(nation);
        snapshot.total = BASE_TRADE_CAPACITY + total_from_ships;
    }
}
