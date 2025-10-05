use bevy::prelude::*;

use super::{goods::Good, stockpile::Stockpile};
use crate::turn_system::TurnPhase;

#[derive(Debug, Clone, Copy)]
pub enum BuildingKind {
    TextileMill,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Building {
    pub kind: BuildingKind,
    pub workers: u8,
}

impl Building {
    pub fn textile_mill(workers: u8) -> Self { Self { kind: BuildingKind::TextileMill, workers } }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// For MVP, we treat buildings attached directly to nation entities and use that nation's Stockpile.
pub fn run_production(
    turn: Res<crate::turn_system::TurnSystem>,
    mut q: Query<(&mut Stockpile, &Building)>,
) {
    if turn.phase != TurnPhase::Processing { return; }

    for (mut stock, building) in q.iter_mut() {
        match building.kind {
            BuildingKind::TextileMill => {
                let workers = building.workers as u32;
                if workers == 0 { continue; }
                let can = stock.get(Good::Wool).min(stock.get(Good::Cotton)).min(workers);
                if can > 0 {
                    // consume inputs
                    let _ = stock.take_up_to(Good::Wool, can);
                    let _ = stock.take_up_to(Good::Cotton, can);
                    // produce outputs
                    stock.add(Good::Cloth, can);
                }
            }
        }
    }
}
