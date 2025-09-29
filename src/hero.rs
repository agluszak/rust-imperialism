use crate::constants::*;
use crate::movement::{ActionPoints, MoveEntityRequest};
use crate::tile_pos::TilePosExt;
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
        self.kills > 0 && self.kills.is_multiple_of(KILLS_PER_HEAL)
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
        sprite.color = if hero.is_selected {
            HERO_COLOR_SELECTED
        } else {
            HERO_COLOR_NORMAL
        };
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

    // Clear all existing preview markers first
    clear_preview_markers(&mut commands, &preview_markers);

    // Find path to preview from selected hero
    if let Some((preview_path, reachable_steps)) = find_active_preview(&hero_query) {
        draw_path_preview(
            &mut commands,
            &preview_path,
            reachable_steps,
            tilemap_size,
            grid_size,
            map_type,
        );
    }
}

/// Clear all path preview markers from the scene
fn clear_preview_markers(
    commands: &mut Commands,
    preview_markers: &Query<Entity, With<PathPreviewMarker>>,
) {
    for entity in preview_markers.iter() {
        commands.entity(entity).despawn();
    }
}

/// Find the active path preview from a selected hero
fn find_active_preview(
    hero_query: &Query<(&Hero, &HeroPathPreview), With<Hero>>,
) -> Option<(Vec<TilePos>, u32)> {
    for (hero, path_preview) in hero_query.iter() {
        if hero.is_selected && !path_preview.planned_path.is_empty() {
            return Some((
                path_preview.planned_path.clone(),
                path_preview.reachable_steps,
            ));
        }
    }
    None
}

/// Draw the path preview markers on the tilemap
fn draw_path_preview(
    commands: &mut Commands,
    preview_path: &[TilePos],
    reachable_steps: u32,
    tilemap_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
) {
    for (i, &pos) in preview_path.iter().enumerate().skip(1) {
        let is_reachable = i <= reachable_steps as usize;
        let is_final = i == preview_path.len() - 1;

        let world_pos =
            pos.to_world_pos_standard(tilemap_size, grid_size, map_type, Z_LAYER_PATH_PREVIEW);

        if is_final {
            spawn_target_marker(commands, world_pos, is_reachable);
        } else {
            spawn_waypoint_marker(commands, world_pos, is_reachable);
        }
    }
}

/// Spawn a target marker at the end of the path
fn spawn_target_marker(
    commands: &mut Commands,
    world_pos: bevy::prelude::Vec3,
    is_reachable: bool,
) {
    let color = if is_reachable {
        TARGET_MARKER_REACHABLE
    } else {
        TARGET_MARKER_UNREACHABLE
    };

    commands.spawn((
        PathPreviewMarker,
        Sprite {
            color,
            custom_size: Some(TARGET_MARKER_SIZE),
            ..default()
        },
        Transform::from_translation(world_pos),
    ));
}

/// Spawn a waypoint marker along the path
fn spawn_waypoint_marker(
    commands: &mut Commands,
    world_pos: bevy::prelude::Vec3,
    is_reachable: bool,
) {
    let color = if is_reachable {
        PATH_PREVIEW_REACHABLE
    } else {
        PATH_PREVIEW_UNREACHABLE
    };

    commands.spawn((
        PathPreviewMarker,
        Sprite {
            color,
            custom_size: Some(PATH_WAYPOINT_SIZE),
            ..default()
        },
        Transform::from_translation(world_pos),
    ));
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
                handle_path_execution(
                    hero_entity,
                    event.target_pos,
                    action_points,
                    &mut path_preview,
                    &mut move_requests,
                    &mut log_writer,
                );
            } else {
                handle_path_preview(
                    *hero_pos,
                    event.target_pos,
                    action_points,
                    &mut path_preview,
                    tilemap_size,
                    &tile_query,
                    tile_storage,
                    &mut log_writer,
                );
            }
            break;
        }
    }
}

/// Handle second click - execute planned path
fn handle_path_execution(
    hero_entity: Entity,
    target_pos: TilePos,
    action_points: &ActionPoints,
    path_preview: &mut HeroPathPreview,
    move_requests: &mut EventWriter<MoveEntityRequest>,
    log_writer: &mut EventWriter<TerminalLogEvent>,
) {
    if action_points.can_move(path_preview.path_cost) {
        move_requests.write(MoveEntityRequest {
            entity: hero_entity,
            target: target_pos,
        });

        log_writer.write(TerminalLogEvent {
            message: format!(
                "Executing path to {:?}, cost: {}, remaining AP: {}",
                target_pos,
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
}

/// Handle first click - show path preview
fn handle_path_preview(
    hero_pos: TilePos,
    target_pos: TilePos,
    action_points: &ActionPoints,
    path_preview: &mut HeroPathPreview,
    tilemap_size: &TilemapSize,
    tile_query: &Query<(&crate::tiles::TileType, &TilePos)>,
    tile_storage: &TileStorage,
    log_writer: &mut EventWriter<TerminalLogEvent>,
) {
    if let Some(path) = crate::pathfinding::PathfindingSystem::find_path_with_combined_query(
        hero_pos,
        target_pos,
        tilemap_size,
        tile_query,
        tile_storage,
    ) {
        let path_cost =
            crate::pathfinding::PathfindingSystem::calculate_path_cost_with_combined_query(
                &path,
                tile_query,
                tile_storage,
            );

        let reachable_steps =
            calculate_reachable_steps(&path, action_points.current, tile_query, tile_storage);

        path_preview.set_path(target_pos, path, path_cost, reachable_steps);

        let message = if action_points.can_move(path_cost) {
            format!(
                "Path to {:?} costs {} AP. Click again to execute.",
                target_pos, path_cost
            )
        } else {
            format!(
                "Path to {:?} costs {} AP (not enough! have {})",
                target_pos, path_cost, action_points.current
            )
        };

        log_writer.write(TerminalLogEvent { message });
    } else {
        log_writer.write(TerminalLogEvent {
            message: format!("No path found to {:?}", target_pos),
        });
        path_preview.clear();
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
