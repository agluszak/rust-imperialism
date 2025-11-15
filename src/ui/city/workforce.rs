use bevy::prelude::*;

use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Capital, PlayerNation, Treasury};
use crate::map::tile_pos::TilePosExt;
use crate::ui::city::components::HireCivilian;

// Note: HireCivilianButton component exists but no UI currently spawns these buttons.
// When hire buttons are added to the UI, they should use the observer pattern:
//
// let civilian_kind = CivilianKind::Engineer;
// .spawn((
//     Button,
//     HireCivilianButton(civilian_kind),
//     observe(move |_: On<Activate>, mut hire_writer: MessageWriter<HireCivilian>| {
//         hire_writer.write(HireCivilian { kind: civilian_kind });
//     }),
// ))

/// Spawn a hired civilian at a suitable location
pub fn spawn_hired_civilian(
    mut commands: Commands,
    mut hire_events: MessageReader<HireCivilian>,
    player_nation: Option<Res<PlayerNation>>,
    nations: Query<(&Capital, &crate::economy::nation::NationId)>,
    mut treasuries: Query<&mut Treasury>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: Query<&Civilian>,
) {
    for event in hire_events.read() {
        let Some(player) = &player_nation else {
            continue;
        };

        // Get capital position
        let Ok((capital, nation_id)) = nations.get(player.entity()) else {
            info!("Cannot hire: no capital found");
            continue;
        };

        // Determine cost based on civilian type
        let cost = match event.kind {
            CivilianKind::Engineer => 200,
            CivilianKind::Prospector => 150,
            CivilianKind::Developer => 180,
            CivilianKind::Miner | CivilianKind::Driller => 120,
            _ => 100,
        };

        // Check if player can afford
        let Ok(mut treasury) = treasuries.get_mut(player.entity()) else {
            continue;
        };

        if treasury.total() < cost {
            info!(
                "Not enough money to hire {:?} (need ${}, have ${})",
                event.kind,
                cost,
                treasury.total()
            );
            continue;
        }

        // Find unoccupied tile near capital
        let spawn_pos = find_unoccupied_tile_near(capital.0, &tile_storage_query, &civilians);

        let Some(spawn_pos) = spawn_pos else {
            info!("No unoccupied tiles near capital to spawn civilian");
            continue;
        };

        // Deduct cost
        treasury.subtract(cost);

        // Spawn civilian
        commands.spawn(Civilian {
            kind: event.kind,
            position: spawn_pos,
            owner: player.entity(),
            owner_id: *nation_id,
            selected: false,
            has_moved: false,
        });

        info!(
            "Hired {:?} for ${} at ({}, {})",
            event.kind, cost, spawn_pos.x, spawn_pos.y
        );
    }
}

/// Find an unoccupied tile near the given position
fn find_unoccupied_tile_near(
    center: bevy_ecs_tilemap::prelude::TilePos,
    tile_storage_query: &Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: &Query<&Civilian>,
) -> Option<bevy_ecs_tilemap::prelude::TilePos> {
    use crate::map::tile_pos::HexExt;

    let center_hex = center.to_hex();

    // Check center first
    if !is_tile_occupied(center, civilians) {
        return Some(center);
    }

    // Check neighbors in expanding rings
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

/// Check if a tile is occupied by a civilian
fn is_tile_occupied(pos: bevy_ecs_tilemap::prelude::TilePos, civilians: &Query<&Civilian>) -> bool {
    civilians.iter().any(|c| c.position == pos)
}
