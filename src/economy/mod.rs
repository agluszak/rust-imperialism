use bevy::prelude::*;

use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::menu::AppState;

pub mod allocation;
pub mod allocation_systems;
pub mod calendar;
pub mod goods;
pub mod market;
pub mod nation;
pub mod production;
pub mod reservation;
pub mod stockpile;
pub mod technology;
pub mod transport;
pub mod treasury;
pub mod workforce;

pub use allocation::{
    AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, Allocations,
    MarketInterest,
};
pub use calendar::{Calendar, Season};
pub use goods::Good;
pub use market::{MARKET_RESOURCES, market_price};
pub use nation::{Capital, Name, NationColor, NationId, NationInstance, PlayerNation};
pub use production::{Building, BuildingKind, ConnectedProduction};
pub use reservation::{ReservationId, ReservationSystem, ResourcePool};
pub use stockpile::Stockpile;
pub use technology::{Technologies, Technology};
pub use transport::{Depot, ImprovementKind, PlaceImprovement, Port, Rails, Roads};
pub use treasury::Treasury;
pub use workforce::{
    RecruitWorkers, RecruitmentCapacity, RecruitmentQueue, TrainWorker, TrainingQueue, Worker,
    WorkerHealth, WorkerSkill, Workforce,
};

/// System set for economy systems that run when in game
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct EconomySet;

/// Plugin that handles all economy-related systems and logic
pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        // Register resources
        app.insert_resource(Calendar::default())
            .insert_resource(transport::Roads::default())
            .insert_resource(transport::Rails::default())
            .insert_resource(production::ConnectedProduction::default())
            .insert_resource(transport::TransportCapacity::default())
            .insert_resource(transport::TransportAllocations::default())
            .insert_resource(transport::TransportDemandSnapshot::default());

        // Register messages
        app.add_message::<transport::PlaceImprovement>()
            .add_message::<transport::RecomputeConnectivity>()
            .add_message::<transport::TransportAdjustAllocation>();

        // Configure the economy system set to run only in-game
        app.configure_sets(Update, EconomySet.run_if(in_state(AppState::InGame)));

        // Core transport and production systems (run every frame when in-game)
        app.add_systems(
            Update,
            (
                transport::apply_improvements,
                transport::compute_rail_connectivity.after(transport::apply_improvements),
                transport::update_transport_capacity.after(transport::compute_rail_connectivity),
                production::calculate_connected_production
                    .after(transport::update_transport_capacity),
                transport::update_transport_demand_snapshot
                    .after(production::calculate_connected_production),
                transport::apply_transport_allocations,
                production::run_production,
            )
                .in_set(EconomySet),
        );

        // Allocation adjustment systems (run every frame when in-game)
        app.add_systems(
            Update,
            (
                workforce::execute_recruitment_orders,
                workforce::execute_training_orders,
                workforce::handle_recruitment,
                workforce::handle_training,
                allocation_systems::apply_recruitment_adjustments,
                allocation_systems::apply_training_adjustments,
                allocation_systems::apply_production_adjustments,
                allocation_systems::apply_market_order_adjustments,
            )
                .in_set(EconomySet),
        );

        // Turn-based economy systems (run when turn changes)
        app.add_systems(
            Update,
            (
                transport::advance_rail_construction
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == TurnPhase::PlayerTurn
                    }),
                workforce::feed_workers
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == TurnPhase::PlayerTurn
                    }),
                allocation_systems::finalize_allocations
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == TurnPhase::Processing
                    }),
                allocation_systems::reset_allocations
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == TurnPhase::PlayerTurn
                    }),
                workforce::update_labor_pools
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == TurnPhase::PlayerTurn
                    }),
            )
                .in_set(EconomySet),
        );
    }
}
