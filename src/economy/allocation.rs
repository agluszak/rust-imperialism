use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::{goods::Good, reservation::ReservationId, workforce::WorkerSkill};

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

    /// Market buy interest: goods the nation wants to buy
    /// (No quantities - just interest flags for market participation)
    pub market_buy_interest: HashSet<Good>,

    /// Market sell allocations: goods the nation wants to sell with quantities
    /// Each ReservationId represents 1 unit reserved for selling
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

    /// Check if nation has buy interest for a good
    pub fn has_buy_interest(&self, good: Good) -> bool {
        self.market_buy_interest.contains(&good)
    }

    /// Get market sell allocation count for a good
    pub fn market_sell_count(&self, good: Good) -> usize {
        self.market_sells.get(&good).map(|v| v.len()).unwrap_or(0)
    }
}
