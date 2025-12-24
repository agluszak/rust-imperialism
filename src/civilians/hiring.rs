use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use crate::civilians::Civilian;
use crate::economy::nation::NationId;
use crate::economy::{Capital, Treasury};
use crate::map::tile_pos::TilePosExt;
use crate::messages::civilians::HireCivilian;

/// Handles [`HireCivilian`] messages for any nation.
///
/// This system is intentionally decoupled from the City UI so that both the
/// player and AI can recruit civilians using the same message flow. UI buttons
/// should send a [`HireCivilian`] message that includes the player's
/// [`NationInstance`](crate::economy::nation::NationInstance).
pub fn spawn_hired_civilian(
    mut commands: Commands,
    mut hire_events: MessageReader<HireCivilian>,
    capitals: Query<(&Capital, &NationId)>,
    mut treasuries: Query<&mut Treasury>,
    tile_storage_query: Query<&TileStorage>,
    civilians: Query<&Civilian>,
) {
    for event in hire_events.read() {
        let nation_entity = event.nation.entity();

        let Ok((capital, _nation_id)) = capitals.get(nation_entity) else {
            info!(
                "Cannot hire {:?} for {:?}: no capital found",
                event.kind, nation_entity
            );
            continue;
        };

        let Some(spawn_pos) = find_unoccupied_tile_near(capital.0, &tile_storage_query, &civilians)
        else {
            info!(
                "Cannot hire {:?} for {:?}: no open tiles near capital",
                event.kind, nation_entity
            );
            continue;
        };

        let Ok(mut treasury) = treasuries.get_mut(nation_entity) else {
            continue;
        };

        let cost = event.kind.hiring_cost();
        if treasury.available() < cost {
            info!(
                "Not enough money to hire {:?} for {:?} (need ${}, have ${})",
                event.kind,
                nation_entity,
                cost,
                treasury.available()
            );
            continue;
        }

        treasury.subtract(cost);

        commands.spawn(Civilian {
            kind: event.kind,
            position: spawn_pos,
            owner: nation_entity,

            has_moved: false,
        });

        info!(
            "Hired {:?} for {:?} at ({}, {})",
            event.kind, nation_entity, spawn_pos.x, spawn_pos.y
        );
    }
}

fn find_unoccupied_tile_near(
    center: TilePos,
    tile_storage_query: &Query<&TileStorage>,
    civilians: &Query<&Civilian>,
) -> Option<TilePos> {
    use crate::map::tile_pos::HexExt;

    let center_hex = center.to_hex();
    if !is_tile_occupied(center, civilians) {
        return Some(center);
    }

    for radius in 1..=3 {
        for neighbor_hex in center_hex.ring(radius) {
            if let Some(neighbor_pos) = neighbor_hex.to_tile_pos()
                && tile_storage_query
                    .iter()
                    .next()
                    .and_then(|storage| storage.get(&neighbor_pos))
                    .is_some()
                && !is_tile_occupied(neighbor_pos, civilians)
            {
                return Some(neighbor_pos);
            }
        }
    }

    None
}

fn is_tile_occupied(pos: TilePos, civilians: &Query<&Civilian>) -> bool {
    civilians.iter().any(|civilian| civilian.position == pos)
}
