use bevy::prelude::Resource;
use std::collections::HashMap;

use crate::economy::Good;

/// List of tradable resources currently exposed in the market UI.
pub const MARKET_RESOURCES: &[Good] = &[
    Good::Grain,
    Good::Fruit,
    Good::Livestock,
    Good::Fish,
    Good::Cotton,
    Good::Wool,
    Good::Timber,
    Good::Coal,
    Good::Iron,
    Good::Oil,
];

/// Aggregated supply and demand information for a single good during a market
/// clearing pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarketVolume {
    pub supply_units: u32,
    pub demand_units: u32,
}

impl MarketVolume {
    pub fn new(supply_units: u32, demand_units: u32) -> Self {
        Self {
            supply_units,
            demand_units,
        }
    }
}

/// Resource responsible for determining prices during market resolution.
///
/// Prices persist between turns and adjust based on supply/demand imbalance.
/// Per the original Imperialism manual: "The prices shown are world market prices
/// from the previous turn. This price is a starting point which may go higher
/// or lower depending on supply and demand."
#[derive(Resource, Debug, Clone)]
pub struct MarketPriceModel {
    base_prices: HashMap<Good, u32>,
    /// Track last turn's supply/demand for each good (for logging/debugging)
    last_volumes: HashMap<Good, MarketVolume>,
}

impl Default for MarketPriceModel {
    fn default() -> Self {
        Self {
            base_prices: default_price_table(),
            last_volumes: HashMap::new(),
        }
    }
}

impl MarketPriceModel {
    /// Returns the trade price for `good`, applying a small premium or discount
    /// based on the provided [`MarketVolume`].
    pub fn price_for(&self, good: Good, volume: MarketVolume) -> u32 {
        let base = self.base_price(good);
        let MarketVolume {
            supply_units,
            demand_units,
        } = volume;

        if supply_units == 0 || demand_units == 0 {
            return base;
        }

        let supply = supply_units as f32;
        let demand = demand_units as f32;
        let total = supply + demand;
        let imbalance = (demand - supply) / total;
        let adjustment_factor = 1.0 + imbalance.clamp(-0.5, 0.5) * 0.25;
        let adjusted = (base as f32 * adjustment_factor).round() as u32;

        adjusted.max(1)
    }

    pub fn set_base_price(&mut self, good: Good, price: u32) {
        self.base_prices.insert(good, price.max(1));
    }

    /// Updates the base price for a good based on observed supply and demand.
    ///
    /// Per the Imperialism manual: "If demand is stronger than supply, price rises.
    /// If supply exceeds demand, price falls. If balanced, price stays similar."
    ///
    /// The adjustment uses a gradual formula to prevent wild price swings:
    /// - Maximum adjustment per turn is ±12.5% of the current price
    /// - Price floors at 20% of original and caps at 300% of original
    pub fn update_price_from_volume(&mut self, good: Good, volume: MarketVolume) {
        self.last_volumes.insert(good, volume);

        let MarketVolume {
            supply_units,
            demand_units,
        } = volume;

        // Only adjust if there was actual market activity
        if supply_units == 0 && demand_units == 0 {
            return;
        }

        let current_price = self.base_price(good);
        let original_price = default_price_table().get(&good).copied().unwrap_or(100);

        // Calculate imbalance: positive = demand > supply (price up), negative = supply > demand (price down)
        let supply = supply_units.max(1) as f32;
        let demand = demand_units.max(1) as f32;
        let imbalance = (demand - supply) / (supply + demand);

        // Apply gradual adjustment: max ±12.5% per turn
        let adjustment_factor = 1.0 + imbalance.clamp(-0.5, 0.5) * 0.25;
        let new_price = (current_price as f32 * adjustment_factor).round() as u32;

        // Clamp to 20%-300% of original price
        let min_price = (original_price as f32 * 0.2).max(1.0) as u32;
        let max_price = (original_price as f32 * 3.0) as u32;
        let clamped_price = new_price.clamp(min_price, max_price);

        if clamped_price != current_price {
            self.base_prices.insert(good, clamped_price);
        }
    }

    /// Returns the current base price for a good (without volume adjustment).
    pub fn current_price(&self, good: Good) -> u32 {
        self.base_price(good)
    }

    /// Returns the last recorded market volume for a good.
    pub fn last_volume(&self, good: Good) -> Option<MarketVolume> {
        self.last_volumes.get(&good).copied()
    }

    fn base_price(&self, good: Good) -> u32 {
        *self.base_prices.get(&good).unwrap_or(&100)
    }
}

fn default_price_table() -> HashMap<Good, u32> {
    let mut map = HashMap::new();
    map.insert(Good::Grain, 60);
    map.insert(Good::Fruit, 60);
    map.insert(Good::Livestock, 80);
    map.insert(Good::Fish, 80);
    map.insert(Good::Cotton, 90);
    map.insert(Good::Wool, 90);
    map.insert(Good::Timber, 70);
    map.insert(Good::Coal, 100);
    map.insert(Good::Iron, 100);
    map.insert(Good::Oil, 110);
    map
}
