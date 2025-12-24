use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::civilians::commands::SelectedCivilian;
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
    selected_civilian: Res<SelectedCivilian>,
    tile_positions: Query<&TilePos>,
    civilians: Query<(Entity, &Civilian)>,
    potential_minerals: Query<&crate::map::PotentialMineral>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    mut order_writer: MessageWriter<CivilianCommand>,
) {
    // Get the clicked tile position
    let Ok(clicked_pos) = tile_positions.get(trigger.entity) else {
        return;
    };

    // Get the selected civilian
    let Some(selected) = selected_civilian.0 else {
        return;
    };
    let Some((civilian_entity, civilian)) = civilians.iter().find(|(entity, _)| *entity == selected) else {
        return;
    };

    let civilian_hex = civilian.position.to_hex();
    let clicked_hex = clicked_pos.to_hex();
    let distance = civilian_hex.distance_to(clicked_hex);

    // If the unit is stationary and supports a tile action, check if action is valid
    if distance == 0 {
        // For prospectors, check if tile can be prospected
        if civilian.kind == CivilianKind::Prospector {
            // Check if tile has PotentialMineral
            let tile_storage = tile_storage_query.iter().next();
            let can_prospect = tile_storage
                .and_then(|storage| storage.get(clicked_pos))
                .and_then(|tile_entity| potential_minerals.get(tile_entity).ok())
                .is_some();

            if can_prospect {
                order_writer.write(CivilianCommand {
                    civilian: civilian_entity,
                    order: CivilianOrderKind::Prospect { to: *clicked_pos },
                });
            }
            // If can't prospect, do nothing (prospector is already on the tile)
            return;
        }

        // For other civilian types, use default tile action
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
        // For prospectors, check if target tile can be prospected
        if civilian.kind == CivilianKind::Prospector {
            let tile_storage = tile_storage_query.iter().next();
            let can_prospect = tile_storage
                .and_then(|storage| storage.get(clicked_pos))
                .and_then(|tile_entity| potential_minerals.get(tile_entity).ok())
                .is_some();

            if can_prospect {
                // Move and prospect
                info!(
                    "Clicked tile ({}, {}) with Prospector, sending move-and-prospect order",
                    clicked_pos.x, clicked_pos.y
                );
                order_writer.write(CivilianCommand {
                    civilian: civilian_entity,
                    order: CivilianOrderKind::Prospect { to: *clicked_pos },
                });
            } else {
                // Just move
                info!(
                    "Clicked tile ({}, {}) with Prospector, sending Move order (no minerals)",
                    clicked_pos.x, clicked_pos.y
                );
                order_writer.write(CivilianCommand {
                    civilian: civilian_entity,
                    order: CivilianOrderKind::Move { to: *clicked_pos },
                });
            }
        } else if let Some(action_order) = civilian.kind.default_tile_action_order(*clicked_pos) {
            // For civilians that support tile actions (farmers, miners, etc.),
            // send their default action order to move-and-improve
            info!(
                "Clicked tile ({}, {}) with {:?}, sending move-and-improve order",
                clicked_pos.x, clicked_pos.y, civilian.kind
            );
            order_writer.write(CivilianCommand {
                civilian: civilian_entity,
                order: action_order,
            });
        } else {
            // For others (Engineers at distance > 1), just move
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
