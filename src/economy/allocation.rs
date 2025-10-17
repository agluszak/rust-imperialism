use bevy::prelude::*;
use std::collections::HashMap;

use super::{goods::Good, reservation::ReservationId, workforce::WorkerSkill};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketOrderKind {
    Buy,
    Sell,
}

/// Per-nation component tracking all resource allocations via reservation IDs
/// Each reservation represents ONE unit of output/worker/etc.
#[derive(Component, Debug, Clone, Default)]
pub struct Allocations {
    /// Production allocations: (building, output_good) -> list of reservations
    /// Each ReservationId represents 1 unit of output
    pub production: HashMap<(Entity, Good), Vec<ReservationId>>,

    /// Recruitment allocations: list of reservations
    /// Each ReservationId represents 1 worker recruitment
    pub recruitment: Vec<ReservationId>,

    /// Training allocations: skill level -> list of reservations
    /// Each ReservationId represents 1 worker training
    pub training: HashMap<WorkerSkill, Vec<ReservationId>>,

    /// Market buy allocations: per-good reservations (each = 1 unit ordered)
    pub market_buys: HashMap<Good, Vec<ReservationId>>,

    /// Market sell allocations: per-good reservations (each = 1 unit offered)
    pub market_sells: HashMap<Good, Vec<ReservationId>>,
}

impl Allocations {
    /// Get production allocation count for a building+output
    pub fn production_count(&self, building: Entity, output: Good) -> usize {
        self.production
            .get(&(building, output))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get recruitment count
    pub fn recruitment_count(&self) -> usize {
        self.recruitment.len()
    }

    /// Get training count for a skill level
    pub fn training_count(&self, skill: WorkerSkill) -> usize {
        self.training.get(&skill).map(|v| v.len()).unwrap_or(0)
    }

    /// Get market buy allocation count for a good
    pub fn market_buy_count(&self, good: Good) -> usize {
        self.market_buys.get(&good).map(|v| v.len()).unwrap_or(0)
    }

    /// Get market sell allocation count for a good
    pub fn market_sell_count(&self, good: Good) -> usize {
        self.market_sells.get(&good).map(|v| v.len()).unwrap_or(0)
    }
}

// ============================================================================
// Messages (Input Layer)
// ============================================================================

/// Player adjusts recruitment allocation (Capitol building)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustRecruitment {
    pub nation: Entity,
    pub requested: u32,
}

/// Player adjusts training allocation (Trade School)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustTraining {
    pub nation: Entity,
    pub from_skill: WorkerSkill,
    pub requested: u32,
}

/// Player adjusts production allocation (mills/factories)
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustProduction {
    pub nation: Entity,
    pub building: Entity,
    pub output_good: Good, // Which output to adjust (Paper, Lumber, etc.)
    pub target_output: u32,
}

/// Player adjusts market buy/sell allocation
#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustMarketOrder {
    pub nation: Entity,
    pub good: Good,
    pub kind: MarketOrderKind,
    pub requested: u32,
}
