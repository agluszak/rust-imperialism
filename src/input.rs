use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::civilians::{Civilian, CivilianKind, CivilianOrderKind, GiveCivilianOrder};
use crate::tile_pos::TilePosExt;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, _app: &mut App) {
        // Tile click handling is done via observers attached to tiles in lib.rs
    }
}

/// Handle tile clicks when an Engineer is selected
pub fn handle_tile_click(
    trigger: On<Pointer<Click>>,
    tile_positions: Query<&TilePos>,
    civilians: Query<(Entity, &Civilian)>,
    mut order_writer: MessageWriter<GiveCivilianOrder>,
) {
    // Get the clicked tile position
    let Ok(clicked_pos) = tile_positions.get(trigger.entity) else {
        return;
    };

    // Find selected Engineer
    let Some((engineer_entity, engineer)) = civilians
        .iter()
        .find(|(_, c)| c.selected && c.kind == CivilianKind::Engineer)
    else {
        return;
    };

    // Check if clicked tile is adjacent to Engineer
    let engineer_hex = engineer.position.to_hex();
    let clicked_hex = clicked_pos.to_hex();
    let distance = engineer_hex.distance_to(clicked_hex);

    if distance == 1 {
        // Adjacent tile: build rail
        info!(
            "Clicked adjacent tile ({}, {}), sending BuildRail order",
            clicked_pos.x, clicked_pos.y
        );

        order_writer.write(GiveCivilianOrder {
            entity: engineer_entity,
            order: CivilianOrderKind::BuildRail { to: *clicked_pos },
        });
    } else if distance > 1 {
        // Non-adjacent tile: move to it
        info!(
            "Clicked non-adjacent tile ({}, {}), sending Move order",
            clicked_pos.x, clicked_pos.y
        );

        order_writer.write(GiveCivilianOrder {
            entity: engineer_entity,
            order: CivilianOrderKind::Move { to: *clicked_pos },
        });
    }
}
