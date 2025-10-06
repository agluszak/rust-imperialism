use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Calendar, PlayerNation, Technologies, Treasury};
use crate::tiles::TerrainType;
use crate::transport_rendering::HoveredTile;
use crate::ui::components::{CalendarDisplay, TileInfoDisplay, TreasuryDisplay, TurnDisplay};
use crate::ui::state::{UIState, UIStateUpdated};

/// Update turn display using centralized UI state
/// This system only runs when UI state has actually changed, reducing overhead
pub fn update_turn_display(
    mut state_events: MessageReader<UIStateUpdated>,
    ui_state: Res<UIState>,
    mut query: Query<&mut Text, With<TurnDisplay>>,
) {
    // Only update when state has changed
    if !state_events.is_empty() {
        state_events.clear(); // Consume all events

        for mut text in query.iter_mut() {
            text.0 = ui_state.turn_display_text();
        }
    }
}

/// Update calendar HUD text when calendar changes or on first frame
pub fn update_calendar_display(
    calendar: Option<Res<Calendar>>,
    mut q: Query<&mut Text, With<CalendarDisplay>>,
) {
    if let Some(calendar) = calendar
        && (calendar.is_changed() || calendar.is_added())
    {
        for mut text in q.iter_mut() {
            text.0 = calendar.display();
        }
    }
}

fn format_currency(value: i64) -> String {
    // naive thousands separator with commas
    let mut s = value.abs().to_string();
    let mut i = s.len() as isize - 3;
    while i > 0 {
        s.insert(i as usize, ',');
        i -= 3;
    }
    if value < 0 {
        format!("-${}", s)
    } else {
        format!("${}", s)
    }
}

/// Update treasury HUD text based on the active player's nation
pub fn update_treasury_display(
    player: Option<Res<PlayerNation>>,
    treasuries: Query<&Treasury>,
    mut q: Query<&mut Text, With<TreasuryDisplay>>,
) {
    if let Some(player) = player
        && let Ok(treasury) = treasuries.get(player.0)
    {
        let s = format_currency(treasury.0);
        for mut text in q.iter_mut() {
            text.0 = s.clone();
        }
    }
}

/// Update tile info display based on hovered tile
pub fn update_tile_info_display(
    hovered_tile: Res<HoveredTile>,
    tile_storage_query: Query<&TileStorage>,
    tile_types: Query<&TerrainType>,
    civilians: Query<&Civilian>,
    player: Option<Res<PlayerNation>>,
    nations: Query<&Technologies>,
    mut display: Query<&mut Text, With<TileInfoDisplay>>,
) {
    if !hovered_tile.is_changed() {
        return;
    }

    for mut text in display.iter_mut() {
        if let Some(tile_pos) = hovered_tile.0 {
            // Find the tile entity and its type
            let mut tile_info = format!("Tile ({}, {})", tile_pos.x, tile_pos.y);

            for tile_storage in tile_storage_query.iter() {
                if let Some(tile_entity) = tile_storage.get(&tile_pos)
                    && let Ok(terrain) = tile_types.get(tile_entity)
                {
                    // Add terrain type
                    let terrain_name = match terrain {
                        TerrainType::Grass => "Grass",
                        TerrainType::Water => "Water",
                        TerrainType::Mountain => "Mountain",
                        TerrainType::Hills => "Hills",
                        TerrainType::Forest => "Forest",
                        TerrainType::Desert => "Desert",
                        TerrainType::Swamp => "Swamp",
                    };
                    tile_info.push_str(&format!("\nTerrain: {}", terrain_name));

                    // If an engineer is selected, show buildability
                    let selected_engineer = civilians
                        .iter()
                        .find(|c| c.selected && c.kind == CivilianKind::Engineer);

                    if selected_engineer.is_some()
                        && let Some(player) = &player
                        && let Ok(techs) = nations.get(player.0)
                    {
                        let buildable = check_buildability(terrain, techs);
                        tile_info.push_str(&format!("\n{}", buildable));
                    }
                }
            }

            text.0 = tile_info;
        } else {
            text.0 = "Hover over a tile".to_string();
        }
    }
}

/// Check if a tile is buildable for rails with current technologies
fn check_buildability(terrain: &TerrainType, technologies: &Technologies) -> String {
    match terrain {
        TerrainType::Mountain => {
            if technologies.has(crate::economy::Technology::MountainEngineering) {
                "Can build rails".to_string()
            } else {
                "⚠ Need Mountain Engineering".to_string()
            }
        }
        TerrainType::Hills => {
            if technologies.has(crate::economy::Technology::HillGrading) {
                "Can build rails".to_string()
            } else {
                "⚠ Need Hill Grading".to_string()
            }
        }
        TerrainType::Swamp => {
            if technologies.has(crate::economy::Technology::SwampDrainage) {
                "Can build rails".to_string()
            } else {
                "⚠ Need Swamp Drainage".to_string()
            }
        }
        _ => "Can build rails".to_string(),
    }
}
