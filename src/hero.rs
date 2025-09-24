use crate::movement::{ActionPoints, MoveEntityRequest};
use crate::turn_system::{TurnPhase, TurnSystem};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::ui::logging::TerminalLogEvent;

#[derive(Component, Debug, Clone)]
pub struct Hero {
    pub name: String,
    pub is_selected: bool,
    pub kills: u32,
}

#[derive(Component, Debug, Clone, Default)]
pub struct HeroPathPreview {
    pub planned_path: Vec<TilePos>,
    pub planned_target: Option<TilePos>,
    pub path_cost: u32,
    pub reachable_steps: u32, // How many steps can be reached with current AP
}

#[derive(Component)]
pub struct HeroSprite;

// Events for hero input
#[derive(Event)]
pub struct HeroSelectionClicked {
    pub target_pos: TilePos,
}

#[derive(Event)]
pub struct HeroMovementClicked {
    pub target_pos: TilePos,
}

#[derive(Component)]
pub struct PathPreviewMarker;

impl Default for Hero {
    fn default() -> Self {
        Self {
            name: "Hero".to_string(),
            is_selected: false,
            kills: 0,
        }
    }
}

impl HeroPathPreview {
    pub fn set_path(
        &mut self,
        target: TilePos,
        path: Vec<TilePos>,
        cost: u32,
        reachable_steps: u32,
    ) {
        self.planned_target = Some(target);
        self.planned_path = path;
        self.path_cost = cost;
        self.reachable_steps = reachable_steps;
    }

    pub fn clear(&mut self) {
        self.planned_target = None;
        self.planned_path.clear();
        self.path_cost = 0;
        self.reachable_steps = 0;
    }

    pub fn has_path_to(&self, target: TilePos) -> bool {
        self.planned_target == Some(target)
    }
}

impl Hero {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_selected: false,
            kills: 0,
        }
    }

    pub fn select(&mut self) {
        self.is_selected = true;
    }

    pub fn deselect(&mut self) {
        self.is_selected = false;
    }

    pub fn add_kill(&mut self) {
        self.kills += 1;
    }

    pub fn should_heal_from_kills(&self) -> bool {
        self.kills > 0 && self.kills.is_multiple_of(3)
    }
}

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HeroSelectionClicked>()
            .add_event::<HeroMovementClicked>()
            .add_systems(
                Update,
                (
                    hero_selection_visual_system,
                    path_preview_visual_system,
                    hero_selection_system,
                    hero_movement_system,
                    refresh_hero_action_points_system,
                ),
            );
    }
}

fn hero_selection_visual_system(
    mut hero_query: Query<(&Hero, &mut Sprite), (With<HeroSprite>, Changed<Hero>)>,
) {
    for (hero, mut sprite) in hero_query.iter_mut() {
        if hero.is_selected {
            sprite.color = Color::srgb(1.0, 1.0, 0.0); // Yellow when selected
        } else {
            sprite.color = Color::srgb(0.0, 0.0, 1.0); // Blue when not selected
        }
    }
}

fn path_preview_visual_system(
    mut commands: Commands,
    hero_query: Query<(&Hero, &HeroPathPreview), With<Hero>>,
    preview_markers: Query<Entity, With<PathPreviewMarker>>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
) {
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    // Check if we need to show path preview
    let mut should_show_preview = false;
    let mut preview_path = Vec::new();
    let mut reachable_steps = 0;

    for (hero, path_preview) in hero_query.iter() {
        if hero.is_selected && !path_preview.planned_path.is_empty() {
            should_show_preview = true;
            preview_path = path_preview.planned_path.clone();
            reachable_steps = path_preview.reachable_steps;
            break;
        }
    }

    // Clear existing markers if no preview should be shown
    if !should_show_preview {
        for entity in preview_markers.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Always clear and redraw to ensure we show the current path
    // Clear existing markers
    for entity in preview_markers.iter() {
        commands.entity(entity).despawn();
    }

    // Draw new path preview with different colors for reachable/unreachable segments
    for (i, &pos) in preview_path.iter().enumerate().skip(1) {
        let is_reachable = i <= reachable_steps as usize;

        if i == preview_path.len() - 1 {
            // Last position - use target marker with color based on reachability
            let world_pos = pos.center_in_world(
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            );

            let target_color = if is_reachable {
                Color::srgba(0.0, 1.0, 0.0, 0.7) // Green for reachable target
            } else {
                Color::srgba(1.0, 0.5, 0.0, 0.7) // Orange for unreachable target
            };

            commands.spawn((
                PathPreviewMarker,
                Sprite {
                    color: target_color,
                    custom_size: Some(Vec2::new(8.0, 8.0)),
                    ..default()
                },
                Transform::from_translation(world_pos.extend(2.0)),
            ));
        } else {
            // Path waypoint with color based on reachability
            let world_pos = pos.center_in_world(
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            );

            let waypoint_color = if is_reachable {
                Color::srgba(1.0, 1.0, 0.0, 0.5) // Yellow for reachable path
            } else {
                Color::srgba(1.0, 0.0, 0.0, 0.5) // Red for unreachable path
            };

            commands.spawn((
                PathPreviewMarker,
                Sprite {
                    color: waypoint_color,
                    custom_size: Some(Vec2::new(4.0, 4.0)),
                    ..default()
                },
                Transform::from_translation(world_pos.extend(2.0)),
            ));
        }
    }
}

// Hero selection system
fn hero_selection_system(
    mut hero_selection_events: EventReader<HeroSelectionClicked>,
    mut hero_query: Query<(&mut Hero, &mut HeroPathPreview, &TilePos), With<Hero>>,
) {
    for event in hero_selection_events.read() {
        for (mut hero, mut path_preview, hero_pos) in hero_query.iter_mut() {
            if *hero_pos == event.target_pos {
                if hero.is_selected {
                    hero.deselect();
                    path_preview.clear();
                } else {
                    hero.select();
                }
                break;
            }
        }
    }
}

// Hero movement system using the unified movement system
fn hero_movement_system(
    mut hero_movement_events: EventReader<HeroMovementClicked>,
    mut hero_query: Query<
        (
            Entity,
            &mut Hero,
            &ActionPoints,
            &mut HeroPathPreview,
            &TilePos,
        ),
        With<Hero>,
    >,
    tile_query: Query<(&crate::tiles::TileType, &TilePos)>,
    tilemap_query: Query<
        (&TilemapSize, &TileStorage, &TilemapGridSize, &TilemapType),
        With<TilemapGridSize>,
    >,
    mut move_requests: EventWriter<MoveEntityRequest>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    let Ok((tilemap_size, tile_storage, _grid_size, _map_type)) = tilemap_query.single() else {
        return;
    };

    for event in hero_movement_events.read() {
        for (hero_entity, hero, action_points, mut path_preview, hero_pos) in hero_query.iter_mut()
        {
            if !hero.is_selected {
                continue;
            }

            if path_preview.has_path_to(event.target_pos) {
                // Second click - execute planned path
                if action_points.can_move(path_preview.path_cost) {
                    // Send movement request to unified system
                    move_requests.write(MoveEntityRequest {
                        entity: hero_entity,
                        target: event.target_pos,
                    });

                    log_writer.write(TerminalLogEvent {
                        message: format!(
                            "Executing path to {:?}, cost: {}, remaining AP: {}",
                            event.target_pos,
                            path_preview.path_cost,
                            action_points.current.saturating_sub(path_preview.path_cost)
                        ),
                    });

                    path_preview.clear();
                } else {
                    log_writer.write(TerminalLogEvent {
                        message: format!(
                            "Not enough action points! Need {}, have {}",
                            path_preview.path_cost, action_points.current
                        ),
                    });
                }
            } else {
                // First click - show path preview
                if let Some(path) =
                    crate::pathfinding::PathfindingSystem::find_path_with_combined_query(
                        *hero_pos,
                        event.target_pos,
                        tilemap_size,
                        &tile_query,
                        tile_storage,
                    )
                {
                    let path_cost = crate::pathfinding::PathfindingSystem::calculate_path_cost_with_combined_query(
                        &path,
                        &tile_query,
                        tile_storage,
                    );

                    // Calculate how many steps are reachable with current action points
                    let reachable_steps = calculate_reachable_steps(
                        &path,
                        action_points.current,
                        &tile_query,
                        tile_storage,
                    );

                    path_preview.set_path(event.target_pos, path, path_cost, reachable_steps);

                    if action_points.can_move(path_cost) {
                        log_writer.write(TerminalLogEvent {
                            message: format!(
                                "Path to {:?} costs {} AP. Click again to execute.",
                                event.target_pos, path_cost
                            ),
                        });
                    } else {
                        log_writer.write(TerminalLogEvent {
                            message: format!(
                                "Path to {:?} costs {} AP (not enough! have {})",
                                event.target_pos, path_cost, action_points.current
                            ),
                        });
                    }
                } else {
                    log_writer.write(TerminalLogEvent {
                        message: format!("No path found to {:?}", event.target_pos),
                    });
                    path_preview.clear();
                }
            }
            break;
        }
    }
}

// Helper function to calculate how many steps can be reached with current action points
fn calculate_reachable_steps(
    path: &[TilePos],
    action_points: u32,
    tile_query: &Query<(&crate::tiles::TileType, &TilePos)>,
    tile_storage: &TileStorage,
) -> u32 {
    let mut accumulated_cost = 0;
    let mut reachable_steps = 0;

    // Skip the first position (current position) and calculate cost for each step
    for (i, &pos) in path.iter().enumerate().skip(1) {
        if let Some(tile_entity) = tile_storage.get(&pos)
            && let Ok((tile_type, _)) = tile_query.get(tile_entity)
        {
            accumulated_cost += tile_type.properties.movement_cost as u32;
            if accumulated_cost <= action_points {
                reachable_steps = i as u32;
            } else {
                break;
            }
        }
    }

    reachable_steps
}

// System to refresh hero action points at the start of each player turn
fn refresh_hero_action_points_system(
    mut hero_query: Query<&mut ActionPoints, With<Hero>>,
    turn_system: Res<TurnSystem>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    // Only refresh on turn changes to PlayerTurn phase
    if turn_system.is_changed() && turn_system.phase == TurnPhase::PlayerTurn {
        for mut action_points in hero_query.iter_mut() {
            let old_ap = action_points.current;
            action_points.refresh();

            if old_ap < action_points.current {
                log_writer.write(TerminalLogEvent {
                    message: format!(
                        "Hero action points refreshed: {}/{}",
                        action_points.current, action_points.max
                    ),
                });
            }
        }
    }
}
