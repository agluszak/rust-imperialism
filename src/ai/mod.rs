use bevy::prelude::*;

use crate::turn_system::{EnemyTurnSet, TurnPhase};

// Simplified AI architecture
pub mod execute;
pub mod markers;
pub mod planner;
pub mod snapshot;

// Public exports
pub use markers::{AiControlledCivilian, AiNation};
pub use planner::{CivilianTask, NationGoal, NationPlan};
pub use snapshot::{AiSnapshot, NationSnapshot};

/// New unified AI plugin using the simplified architecture.
///
/// This plugin runs all AI logic once per turn in OnEnter(EnemyTurn):
/// 1. Build snapshot of game state
/// 2. Generate plans for each AI nation
/// 3. Execute plans by sending orders
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<snapshot::AiSnapshot>().add_systems(
            OnEnter(TurnPhase::EnemyTurn),
            (snapshot::build_ai_snapshot, execute::execute_ai_turn)
                .chain()
                .in_set(EnemyTurnSet::Actions),
        );
    }
}
