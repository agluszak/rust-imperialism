use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::combat::HeroAttackClicked;
use crate::hero::{Hero, HeroMovementClicked, HeroSelectionClicked};
use crate::monster::Monster;
use crate::tiles::{TerrainType, TileCategory, TileType};
use crate::turn_system::TurnSystem;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TerrainCycleClicked>()
            .add_systems(Update, terrain_editing_system);
    }
}

// Terrain editing event
#[derive(Message)]
pub struct TerrainCycleClicked {
    pub target_entity: Entity,
}

// Main input dispatcher - converts clicks to specific events
pub fn handle_tile_click(
    trigger: On<Pointer<Click>>,
    mut hero_selection_events: MessageWriter<HeroSelectionClicked>,
    mut hero_combat_events: MessageWriter<HeroAttackClicked>,
    mut hero_movement_events: MessageWriter<HeroMovementClicked>,
    mut terrain_edit_events: MessageWriter<TerrainCycleClicked>,
    tile_query: Query<(&TileType, &TilePos)>,
    hero_query: Query<(&Hero, &TilePos), With<Hero>>,
    monster_query: Query<&TilePos, With<Monster>>,
    turn_system: Res<TurnSystem>,
) {
    let ev = trigger.event();
    let entity = ev.event_target();
    let pointer_button = ev.button;

    // Get the clicked tile position
    let Ok((_, target_pos)) = tile_query.get(entity) else {
        return;
    };

    match pointer_button {
        PointerButton::Primary => {
            if !turn_system.is_player_turn() {
                return;
            }

            // Check what's at the target position and send appropriate event
            if hero_query
                .iter()
                .any(|(_, hero_pos)| *hero_pos == *target_pos)
            {
                // Clicking on hero - selection
                hero_selection_events.write(HeroSelectionClicked {
                    target_pos: *target_pos,
                });
            } else if monster_query
                .iter()
                .any(|monster_pos| *monster_pos == *target_pos)
            {
                // Clicking on monster - combat
                hero_combat_events.write(HeroAttackClicked {
                    target_pos: *target_pos,
                });
            } else {
                // Clicking on empty tile - movement
                hero_movement_events.write(HeroMovementClicked {
                    target_pos: *target_pos,
                });
            }
        }
        PointerButton::Secondary => {
            // Right click - terrain editing
            terrain_edit_events.write(TerrainCycleClicked {
                target_entity: entity,
            });
        }
        _ => {}
    }
}

// Terrain editing system
fn terrain_editing_system(
    mut terrain_edit_events: MessageReader<TerrainCycleClicked>,
    mut tile_query: Query<(&mut TileTextureIndex, &mut TileType, &TilePos)>,
) {
    for event in terrain_edit_events.read() {
        if let Ok((mut texture_index, mut tile_type, _)) = tile_query.get_mut(event.target_entity) {
            let new_terrain = match &tile_type.category {
                TileCategory::Terrain(TerrainType::Grass) => TerrainType::Water,
                TileCategory::Terrain(TerrainType::Water) => TerrainType::Mountain,
                TileCategory::Terrain(TerrainType::Mountain) => TerrainType::Forest,
                TileCategory::Terrain(TerrainType::Forest) => TerrainType::Desert,
                TileCategory::Terrain(TerrainType::Desert) => TerrainType::Grass,
                _ => TerrainType::Grass,
            };

            *tile_type = TileType::terrain(new_terrain);
            texture_index.0 = tile_type.get_texture_index();
        }
    }
}
