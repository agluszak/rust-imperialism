use bevy::prelude::*;

use super::components::{
    HireCivilian, HireCivilianButton, RecruitWorkersButton, TrainWorkerButton,
};
use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Capital, PlayerNation, RecruitWorkers, TrainWorker, Treasury};
use crate::tile_pos::TilePosExt;
use crate::ui::logging::TerminalLogEvent;

/// Handle hire civilian button clicks
pub fn handle_hire_button_clicks(
    interactions: Query<(&Interaction, &HireCivilianButton), Changed<Interaction>>,
    mut hire_writer: MessageWriter<HireCivilian>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            info!("Hire {:?} button clicked", button.0);
            hire_writer.write(HireCivilian { kind: button.0 });
        }
    }
}

/// Spawn a hired civilian at a suitable location
pub fn spawn_hired_civilian(
    mut commands: Commands,
    mut hire_events: MessageReader<HireCivilian>,
    player_nation: Option<Res<PlayerNation>>,
    nations: Query<&Capital>,
    mut treasuries: Query<&mut Treasury>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: Query<&Civilian>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for event in hire_events.read() {
        let Some(player) = &player_nation else {
            continue;
        };

        // Get capital position
        let Ok(capital) = nations.get(player.entity()) else {
            log_events.write(TerminalLogEvent {
                message: "Cannot hire: no capital found".to_string(),
            });
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
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Not enough money to hire {:?} (need ${}, have ${})",
                    event.kind,
                    cost,
                    treasury.total()
                ),
            });
            continue;
        }

        // Find unoccupied tile near capital
        let spawn_pos = find_unoccupied_tile_near(capital.0, &tile_storage_query, &civilians);

        let Some(spawn_pos) = spawn_pos else {
            log_events.write(TerminalLogEvent {
                message: "No unoccupied tiles near capital to spawn civilian".to_string(),
            });
            continue;
        };

        // Deduct cost
        treasury.subtract(cost);

        // Spawn civilian
        commands.spawn(Civilian {
            kind: event.kind,
            position: spawn_pos,
            owner: player.entity(),
            selected: false,
            has_moved: false,
        });

        log_events.write(TerminalLogEvent {
            message: format!(
                "Hired {:?} for ${} at ({}, {})",
                event.kind, cost, spawn_pos.x, spawn_pos.y
            ),
        });
    }
}

/// Find an unoccupied tile near the given position
fn find_unoccupied_tile_near(
    center: bevy_ecs_tilemap::prelude::TilePos,
    tile_storage_query: &Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: &Query<&Civilian>,
) -> Option<bevy_ecs_tilemap::prelude::TilePos> {
    use crate::tile_pos::HexExt;

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

/// Handle recruit workers button clicks (Input Layer)
pub fn handle_recruit_workers_buttons(
    interactions: Query<(&Interaction, &RecruitWorkersButton), Changed<Interaction>>,
    mut writer: MessageWriter<RecruitWorkers>,
    player_nation: Option<Res<PlayerNation>>,
    buttons: Query<Entity, With<RecruitWorkersButton>>,
) {
    // Debug: check if buttons exist
    let button_count = buttons.iter().count();
    if button_count > 0 {
        trace!("Found {} recruit buttons in scene", button_count);
    }

    let Some(player_nation) = player_nation else {
        warn!("No player nation found for recruitment");
        return;
    };

    for (interaction, button) in interactions.iter() {
        debug!("Recruit button interaction: {:?}", interaction);
        if *interaction == Interaction::Pressed {
            info!("Recruit {} workers button clicked", button.count);
            writer.write(RecruitWorkers {
                nation: player_nation.instance(),
                count: button.count,
            });
        }
    }
}

/// Handle train worker button clicks (Input Layer)
pub fn handle_train_worker_buttons(
    interactions: Query<(&Interaction, &TrainWorkerButton), Changed<Interaction>>,
    mut writer: MessageWriter<TrainWorker>,
    player_nation: Option<Res<PlayerNation>>,
    buttons: Query<Entity, With<TrainWorkerButton>>,
) {
    // Debug: check if buttons exist
    let button_count = buttons.iter().count();
    if button_count > 0 {
        trace!("Found {} train buttons in scene", button_count);
    }

    let Some(player_nation) = player_nation else {
        warn!("No player nation found for training");
        return;
    };

    for (interaction, button) in interactions.iter() {
        debug!("Train button interaction: {:?}", interaction);
        if *interaction == Interaction::Pressed {
            info!("Train worker button clicked: {:?}", button.from_skill);
            writer.write(TrainWorker {
                nation: player_nation.instance(),
                from_skill: button.from_skill,
            });
        }
    }
}
