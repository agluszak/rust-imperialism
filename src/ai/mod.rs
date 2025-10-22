use bevy::prelude::*;

use crate::ui::menu::AppState;

pub mod behavior;
pub mod context;
pub mod markers;

pub use behavior::AiBehaviorPlugin;
pub use context::{
    AiAllocationSnapshot, AiNationSnapshot, AiStockpileEntry, AiTransportAllocation,
    AiTransportDemand, AiTransportSnapshot, AiTurnContext, AiWorkforceSnapshot, enemy_turn_entered,
    populate_ai_turn_context,
};
pub use markers::{AiControlledCivilian, AiNation};

/// Registers shared AI infrastructure such as the per-turn context cache.
pub struct AiSupportPlugin;

impl Plugin for AiSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiTurnContext>().add_systems(
            Update,
            populate_ai_turn_context
                .run_if(in_state(AppState::InGame))
                .run_if(enemy_turn_entered),
        );
    }
}
