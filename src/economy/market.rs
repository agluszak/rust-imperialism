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

/// Placeholder pricing table for the market screen.
///
/// Prices are intentionally simple for the initial UI and roughly mirror
/// Imperialism's early-game values. They will be replaced by dynamic market
/// clearing logic in a later milestone.
pub fn market_price(good: Good) -> u32 {
    match good {
        Good::Grain | Good::Fruit => 60,
        Good::Livestock | Good::Fish => 80,
        Good::Cotton | Good::Wool => 90,
        Good::Timber => 70,
        Good::Coal | Good::Iron => 100,
        Good::Oil => 110,
        _ => 100,
    }
}
