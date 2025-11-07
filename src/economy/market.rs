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
/// The current implementation uses a fixed price table but the dedicated
/// resource allows future systems to mutate the table or compute prices based
/// on the provided [`MarketVolume`] data.
#[derive(Resource, Debug, Clone)]
pub struct MarketPriceModel {
    base_prices: HashMap<Good, u32>,
}

impl Default for MarketPriceModel {
    fn default() -> Self {
        Self {
            base_prices: default_price_table(),
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
