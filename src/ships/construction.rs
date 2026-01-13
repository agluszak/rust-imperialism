use bevy::prelude::*;

use crate::economy::Good;
use crate::economy::nation::{NationInstance, OwnedBy};
use crate::economy::stockpile::Stockpile;
use crate::ships::types::{Ship, ShipKind};

/// Message to request ship construction
#[derive(Message, Debug, Clone, Copy)]
pub struct ConstructShip {
    pub nation: NationInstance,
    pub kind: ShipKind,
}

/// System to process ship construction at the end of processing phase
/// This replaces the Good::Ship production in the shipyard
pub fn construct_ships_from_production(
    mut commands: Commands,
    mut nations: Query<(Entity, &mut Stockpile)>,
) {
    for (nation_entity, mut stockpile) in nations.iter_mut() {
        // Check for materials to build ships (Steel, Lumber, Fuel)
        let steel = stockpile.get(Good::Steel);
        let lumber = stockpile.get(Good::Lumber);
        let fuel = stockpile.get(Good::Fuel);

        // Calculate how many ships can be built
        let can_build = steel.min(lumber).min(fuel);

        if can_build > 0 {
            // Consume materials
            let actually_built = stockpile.take_up_to(Good::Steel, can_build);
            stockpile.take_up_to(Good::Lumber, actually_built);
            stockpile.take_up_to(Good::Fuel, actually_built);

            // Spawn ship entities (use entity ID as the unique identifier)
            for i in 0..actually_built {
                let ship_entity = commands
                    .spawn((
                        Ship::new(ShipKind::Trader, nation_entity),
                        OwnedBy(nation_entity),
                        Name::new(format!("Trade Ship #{}", i + 1)),
                    ))
                    .id();

                info!(
                    "Constructed ship {:?} for nation {:?}",
                    ship_entity, nation_entity
                );
            }
        }
    }
}
