use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::transport::ImprovementKind;

/// Message to place a transport improvement between two tiles.
#[derive(Message, Debug, Clone, Copy)]
pub struct PlaceImprovement {
    pub a: TilePos,
    pub b: TilePos,
    pub kind: ImprovementKind,
    pub engineer: Option<Entity>,
}

/// Message to trigger rail network connectivity recomputation after topology changes.
#[derive(Message, Debug, Clone, Copy)]
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
