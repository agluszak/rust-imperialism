use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::transport::ImprovementKind;

/// Event to place a transport improvement between two tiles.
/// Triggered via `commands.trigger(PlaceImprovement { ... })`.
#[derive(Event, Debug, Clone, Copy)]
pub struct PlaceImprovement {
    pub a: TilePos,
    pub b: TilePos,
    pub kind: ImprovementKind,
    pub nation: Option<Entity>,
    pub engineer: Option<Entity>,
}

/// Event to trigger rail network connectivity recomputation after topology changes.
/// Triggered via `commands.trigger(RecomputeConnectivity)`.
#[derive(Event, Debug, Clone, Copy)]
pub struct RecomputeConnectivity;

#[cfg(test)]
mod tests {
    use crate::messages::*;

    #[test]
    fn transport_messages_are_send_sync() {
        fn assert_message<T: Send + Sync + 'static>() {}

        assert_message::<PlaceImprovement>();
        assert_message::<RecomputeConnectivity>();
    }
}
