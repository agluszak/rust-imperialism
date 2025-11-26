use bevy::prelude::*;

use crate::turn_system::{EnemyTurnSet, TurnPhase};

pub mod behavior;
pub mod context;
pub mod markers;
pub mod trade;

pub use behavior::AiBehaviorPlugin;
pub use context::{
    AiAllocationSnapshot, AiMarketBuy, AiNationSnapshot, AiPlanLedger, AiStockpileEntry,
    AiTransportAllocation, AiTransportDemand, AiTransportSnapshot, AiTurnContext,
    AiWorkforceSnapshot, BeliefState, MacroActionCandidate, MacroTag, MarketView, MinorId,
    TransportAnalysis, TurnCandidates, gather_turn_candidates, populate_ai_turn_context,
    update_belief_state_system, update_market_view_system, update_transport_analysis_system,
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
            // Populate AI turn context once when entering EnemyTurn
            .add_systems(
                OnEnter(TurnPhase::EnemyTurn),
                populate_ai_turn_context.in_set(EnemyTurnSet::Setup),
            );
    }
}
