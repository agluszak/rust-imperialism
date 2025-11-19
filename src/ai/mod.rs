use bevy::prelude::*;

use crate::ui::menu::AppState;

pub mod behavior;
pub mod context;
pub mod markers;
pub mod trade;

pub use behavior::AiBehaviorPlugin;
pub use context::{
    AiAllocationSnapshot, AiMarketBuy, AiNationSnapshot, AiPlanLedger, AiStockpileEntry,
    AiTransportAllocation, AiTransportDemand, AiTransportSnapshot, AiTurnContext,
    AiWorkforceSnapshot, BeliefState, MacroActionCandidate, MacroTag, MarketView, MinorId,
    TransportAnalysis, TurnCandidates, enemy_turn_entered, gather_turn_candidates,
    populate_ai_turn_context, update_belief_state_system, update_market_view_system,
    update_transport_analysis_system,
};
pub use markers::{AiControlledCivilian, AiNation};
pub use trade::AiEconomyPlugin;

/// Registers shared AI infrastructure such as the per-turn context cache.
pub struct AiSupportPlugin;

impl Plugin for AiSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiTurnContext>()
            .init_resource::<BeliefState>()
            .init_resource::<MarketView>()
            .init_resource::<TransportAnalysis>()
            .init_resource::<TurnCandidates>()
            .init_resource::<AiPlanLedger>()
            .add_systems(
                Update,
                populate_ai_turn_context
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_entered),
            );
    }
}
