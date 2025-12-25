use bevy::prelude::*;

use crate::orders::OrdersQueue;
use crate::turn_system::{PlayerTurnSet, ProcessingSet, TurnPhase};
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
pub mod trade;
pub mod trade_capacity;
pub mod transport;
pub mod treasury;
pub mod workforce;

pub use crate::messages::{
    AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
};
pub use allocation::Allocations;
pub use calendar::{Calendar, Season};
pub use goods::Good;
pub use market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
pub use nation::{Capital, Nation, NationColor, NationInstance, PlayerNation};
pub use production::{Building, BuildingKind, ConnectedProduction};
pub use reservation::{ReservationId, ReservationSystem, ResourcePool};
pub use stockpile::Stockpile;
pub use technology::{Technologies, Technology};
pub use trade_capacity::{TradeCapacity, TradeCapacitySnapshot};
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
            .insert_resource(market::MarketPriceModel::default())
            .insert_resource(transport::Roads::default())
            .insert_resource(transport::Rails::default())
            .insert_resource(production::ConnectedProduction::default())
            .insert_resource(transport::TransportCapacity::default())
            .insert_resource(trade_capacity::TradeCapacity::default())
            .insert_resource(transport::TransportAllocations::default())
            .insert_resource(transport::TransportDemandSnapshot::default())
            .insert_resource(OrdersQueue::default());

        // Register messages
        app.add_message::<transport::PlaceImprovement>()
            .add_message::<transport::RecomputeConnectivity>()
            .add_message::<transport::TransportAdjustAllocation>()
            .add_message::<AdjustRecruitment>()
            .add_message::<AdjustTraining>()
            .add_message::<AdjustProduction>()
            .add_message::<AdjustMarketOrder>()
            .add_message::<RecruitWorkers>()
            .add_message::<TrainWorker>();

        // Configure the economy system set to run only in-game
        app.configure_sets(Update, EconomySet.run_if(in_state(AppState::InGame)));

        // ====================================================================
        // Core systems that run every frame (Update schedule)
        // ====================================================================

        // Transport and connectivity (must run every frame to track changes)
        app.add_systems(
            Update,
            (
                transport::initialize_transport_capacity,
                trade_capacity::initialize_trade_capacity,
                transport::apply_improvements,
                transport::compute_rail_connectivity.after(transport::apply_improvements),
                production::calculate_connected_production
                    .after(transport::compute_rail_connectivity),
                transport::update_transport_demand_snapshot
                    .after(production::calculate_connected_production),
                transport::apply_transport_allocations,
            )
                .in_set(EconomySet),
        );

        // Allocation adjustment systems (player can adjust during their turn)
        app.add_systems(
            Update,
            (
                workforce::execute_recruitment_orders,
                workforce::execute_training_orders,
                workforce::handle_recruitment,
                workforce::handle_training,
                (
                    allocation_systems::apply_recruitment_adjustments,
                    allocation_systems::apply_training_adjustments,
                    allocation_systems::apply_production_adjustments,
                    allocation_systems::apply_market_order_adjustments,
                )
                    .chain(),
            )
                .in_set(EconomySet),
        );

        // Execute queued orders (run every frame, but only when queue is not empty)
        app.add_systems(
            Update,
            (
                allocation_systems::execute_queued_recruitment_orders,
                allocation_systems::execute_queued_training_orders,
                allocation_systems::execute_queued_production_orders,
                allocation_systems::execute_queued_transport_orders,
                allocation_systems::execute_queued_market_orders,
            )
                .chain()
                .run_if(|orders: Res<OrdersQueue>| !orders.is_empty())
                .in_set(EconomySet),
        );

        // ====================================================================
        // PlayerTurn phase systems (OnEnter - run once when phase starts)
        // ====================================================================

        // Collection: Gather resources from transport network
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            (
                transport::advance_rail_construction,
                production::collect_connected_production,
            )
                .in_set(PlayerTurnSet::Collection),
        );

        // Maintenance: Feed workers, apply recurring effects
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            (workforce::feed_workers, workforce::update_labor_pools)
                .in_set(PlayerTurnSet::Maintenance),
        );

        // Market: Resolve orders from previous turn
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            trade::resolve_market_orders.in_set(PlayerTurnSet::Market),
        );

        // Reset: Clear allocations for new turn
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            allocation_systems::reset_allocations.in_set(PlayerTurnSet::Reset),
        );

        // ====================================================================
        // Processing phase systems (OnEnter - run once when phase starts)
        // ====================================================================

        // Finalize: Commit reservations
        app.add_systems(
            OnEnter(TurnPhase::Processing),
            allocation_systems::finalize_allocations.in_set(ProcessingSet::Finalize),
        );

        // Production: Execute production
        app.add_systems(
            OnEnter(TurnPhase::Processing),
            production::run_production.in_set(ProcessingSet::Production),
        );

        // Conversion: Convert goods to capacity
        app.add_systems(
            OnEnter(TurnPhase::Processing),
            (
                transport::convert_transport_goods_to_capacity,
                trade_capacity::convert_ships_to_trade_capacity,
            )
                .in_set(ProcessingSet::Conversion),
        );
    }
}
