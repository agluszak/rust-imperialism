use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::civilians::{Civilian, CivilianCommand, CivilianKind, CivilianOrderKind};
use crate::map::tile_pos::TilePosExt;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, _app: &mut App) {
        // Tile click handling is done via observers attached to tiles in lib.rs
    }
}

/// Handle tile clicks when any civilian is selected
pub fn handle_tile_click(
    trigger: On<Pointer<Click>>,
    tile_positions: Query<&TilePos>,
    civilians: Query<(Entity, &Civilian)>,
    mut order_writer: MessageWriter<CivilianCommand>,
) {
    // Get the clicked tile position
    let Ok(clicked_pos) = tile_positions.get(trigger.entity) else {
        return;
    };

    // Find any selected civilian
    let Some((civilian_entity, civilian)) = civilians.iter().find(|(_, c)| c.selected) else {
        return;
    };

    let civilian_hex = civilian.position.to_hex();
    let clicked_hex = clicked_pos.to_hex();
    let distance = civilian_hex.distance_to(clicked_hex);

    // If the unit is stationary and supports a tile action, trigger it directly
    if distance == 0 {
        if let Some(order) = civilian.kind.default_tile_action_order(*clicked_pos) {
            order_writer.write(CivilianCommand {
                civilian: civilian_entity,
                order,
            });
        }
        return;
    }

    // Special handling for Engineer: adjacent click = build rail
    if civilian.kind == CivilianKind::Engineer && distance == 1 {
        info!(
            "Clicked adjacent tile ({}, {}) with Engineer, sending BuildRail order",
            clicked_pos.x, clicked_pos.y
        );

        order_writer.write(CivilianCommand {
            civilian: civilian_entity,
            order: CivilianOrderKind::BuildRail { to: *clicked_pos },
        });
    } else if distance >= 1 {
        // For civilians that support tile actions (farmers, prospectors, miners, etc.),
        // send their default action order to move-and-improve
        // For others (Engineers at distance > 1), just move
        if let Some(action_order) = civilian.kind.default_tile_action_order(*clicked_pos) {
            info!(
                "Clicked tile ({}, {}) with {:?}, sending move-and-improve order",
                clicked_pos.x, clicked_pos.y, civilian.kind
            );
            order_writer.write(CivilianCommand {
                civilian: civilian_entity,
                order: action_order,
            });
        } else {
            info!(
                "Clicked tile ({}, {}) with {:?}, sending Move order",
                clicked_pos.x, clicked_pos.y, civilian.kind
            );
            order_writer.write(CivilianCommand {
                civilian: civilian_entity,
                order: CivilianOrderKind::Move { to: *clicked_pos },
            });
        }
    }
}
