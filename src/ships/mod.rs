use bevy::prelude::*;

use crate::turn_system::TurnPhase;

pub mod construction;
pub mod types;

pub use types::{NextShipId, Ship, ShipId, ShipKind};

/// Plugin for ship management
pub struct ShipsPlugin;

impl Plugin for ShipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NextShipId>()
            .register_type::<ShipId>()
            .register_type::<Ship>()
            .add_systems(
                OnEnter(TurnPhase::PlayerTurn),
                reset_ship_movement_flags,
            )
            .add_systems(
                OnEnter(TurnPhase::Processing),
                construction::construct_ships_from_production,
            );
    }
}

/// Reset has_moved flags at the start of each turn
fn reset_ship_movement_flags(mut ships: Query<&mut Ship>) {
    for mut ship in ships.iter_mut() {
        ship.has_moved = false;
    }
}

/// Count ships owned by a nation
pub fn count_ships_for_nation(ships: &Query<&Ship>, nation: Entity) -> usize {
    ships.iter().filter(|ship| ship.owner == nation).count()
}
