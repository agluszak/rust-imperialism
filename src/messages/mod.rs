pub mod civilians;
pub mod diplomacy;
pub mod economy;
pub mod transport;
pub mod workforce;

pub use civilians::{CivilianCommand, CivilianCommandError, CivilianCommandRejected};
pub use diplomacy::{DiplomaticOrder, DiplomaticOrderKind};
pub use economy::{
    AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
};
pub use transport::{PlaceImprovement, RecomputeConnectivity};
pub use workforce::{RecruitWorkers, TrainWorker};

// Messages currently live alongside their originating subsystems. This module
// re-exports them behind a unified namespace so that future AI systems can
// depend on the same message definitions as the player-facing UI without
// coupling to specific subsystem implementations.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_messages_are_send_sync_static() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}

        assert_send_sync_static::<AdjustRecruitment>();
        assert_send_sync_static::<AdjustTraining>();
        assert_send_sync_static::<AdjustProduction>();
        assert_send_sync_static::<AdjustMarketOrder>();
        assert_send_sync_static::<RecruitWorkers>();
        assert_send_sync_static::<TrainWorker>();
        assert_send_sync_static::<PlaceImprovement>();
        assert_send_sync_static::<RecomputeConnectivity>();
        assert_send_sync_static::<DiplomaticOrder>();
        assert_send_sync_static::<CivilianCommand>();
        assert_send_sync_static::<CivilianCommandRejected>();
    }
}
