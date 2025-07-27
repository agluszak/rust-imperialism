use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::collections::VecDeque;

#[derive(Component, Debug, Clone)]
pub struct Hero {
    pub name: String,
    pub movement_points: u32,
    pub max_movement_points: u32,
    pub is_selected: bool,
    pub kills: u32,
}

#[derive(Component, Debug, Clone)]
pub struct HeroMovement {
    pub path: VecDeque<TilePos>,
    pub movement_speed: f32,
    pub is_moving: bool,
    pub target_world_pos: Option<Vec3>,
}

#[derive(Component, Debug, Clone, Default)]
pub struct HeroPathPreview {
    pub planned_path: Vec<TilePos>,
    pub planned_target: Option<TilePos>,
    pub path_cost: u32,
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
            movement_points: 3,
            max_movement_points: 3,
            is_selected: false,
            kills: 0,
        }
    }
}

impl Default for HeroMovement {
    fn default() -> Self {
        Self {
            path: VecDeque::new(),
            movement_speed: 200.0,
            is_moving: false,
            target_world_pos: None,
        }
    }
}

impl HeroPathPreview {
    pub fn set_path(&mut self, target: TilePos, path: Vec<TilePos>, cost: u32) {
        self.planned_target = Some(target);
        self.planned_path = path;
        self.path_cost = cost;
    }

    pub fn clear(&mut self) {
        self.planned_target = None;
        self.planned_path.clear();
        self.path_cost = 0;
    }

    pub fn has_path_to(&self, target: TilePos) -> bool {
        self.planned_target == Some(target)
    }
}

impl Hero {
    pub fn new(name: String, movement_points: u32) -> Self {
        Self {
            name,
            movement_points,
            max_movement_points: movement_points,
            is_selected: false,
            kills: 0,
        }
    }

    pub fn can_move(&self, distance: u32) -> bool {
        self.movement_points >= distance
    }

    pub fn consume_movement(&mut self, distance: u32) {
        self.movement_points = self.movement_points.saturating_sub(distance);
    }

    pub fn refresh_movement(&mut self) {
        self.movement_points = self.max_movement_points;
    }

    pub fn select(&mut self) {
        self.is_selected = true;
    }

    pub fn deselect(&mut self) {
        self.is_selected = false;
    }

    pub fn can_attack(&self) -> bool {
        self.movement_points >= 1
    }

    pub fn attack(&mut self) -> bool {
        if self.can_attack() {
            self.movement_points -= 1;
            true
        } else {
            false
        }
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
                    hero_movement_animation_system,
                    hero_selection_visual_system,
                    path_preview_visual_system,
                    hero_selection_system,
                    hero_movement_system,
                ),
            );
    }
}

fn hero_movement_animation_system(
    time: Res<Time>,
    mut hero_query: Query<(&mut Transform, &mut HeroMovement, &mut TilePos), With<Hero>>,
    tilemap_query: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
    )>,
) {
    let Ok((tilemap_size, grid_size, tile_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for (mut transform, mut movement, mut hero_pos) in hero_query.iter_mut() {
        if !movement.is_moving {
            continue;
        }

        if let Some(target_pos) = movement.target_world_pos {
            let direction = target_pos - transform.translation;
            let distance = direction.length();

            if distance < 5.0 {
                transform.translation = target_pos;
                movement.is_moving = false;
                movement.target_world_pos = None;

                // Update logical position
                if let Some(next_tile) = movement.path.pop_front() {
                    *hero_pos = next_tile;
                }

                if !movement.path.is_empty()
                    && let Some(next_tile) = movement.path.front()
                {
                    movement.target_world_pos = Some(
                        next_tile
                            .center_in_world(
                                tilemap_size,
                                grid_size,
                                tile_size,
                                map_type,
                                &TilemapAnchor::Center,
                            )
                            .extend(1.0),
                    );
                    movement.is_moving = true;
                }
            } else {
                let move_direction = direction.normalize();
                transform.translation +=
                    move_direction * movement.movement_speed * time.delta_secs();
            }
        }
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

    for (hero, path_preview) in hero_query.iter() {
        if hero.is_selected && !path_preview.planned_path.is_empty() {
            should_show_preview = true;
            preview_path = path_preview.planned_path.clone();
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

    // Draw new path preview
    for (i, &pos) in preview_path.iter().enumerate().skip(1) {
        if i == preview_path.len() - 1 {
            // Last position - use target marker
            let world_pos = pos.center_in_world(
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            );

            commands.spawn((
                PathPreviewMarker,
                Sprite {
                    color: Color::srgba(0.0, 1.0, 0.0, 0.7), // Semi-transparent green for target
                    custom_size: Some(Vec2::new(8.0, 8.0)),
                    ..default()
                },
                Transform::from_translation(world_pos.extend(2.0)),
            ));
        } else {
            // Path waypoint
            let world_pos = pos.center_in_world(
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            );

            commands.spawn((
                PathPreviewMarker,
                Sprite {
                    color: Color::srgba(1.0, 1.0, 0.0, 0.5), // Semi-transparent yellow for path
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

// Hero movement system
fn hero_movement_system(
    mut hero_movement_events: EventReader<HeroMovementClicked>,
    mut hero_query: Query<
        (
            Entity,
            &mut Hero,
            &mut HeroMovement,
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
) {
    let Ok((tilemap_size, tile_storage, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for event in hero_movement_events.read() {
        for (_hero_entity, mut hero, mut hero_movement, mut path_preview, hero_pos) in
            hero_query.iter_mut()
        {
            if !hero.is_selected {
                continue;
            }

            if path_preview.has_path_to(event.target_pos) {
                // Second click - execute planned path
                if hero.can_move(path_preview.path_cost) {
                    hero.consume_movement(path_preview.path_cost);

                    hero_movement.path = path_preview.planned_path.clone().into();

                    if let Some(first_step) = hero_movement.path.front() {
                        hero_movement.target_world_pos = Some(
                            first_step
                                .center_in_world(
                                    tilemap_size,
                                    grid_size,
                                    &TilemapTileSize { x: 16.0, y: 16.0 },
                                    map_type,
                                    &TilemapAnchor::Center,
                                )
                                .extend(1.0),
                        );
                        hero_movement.is_moving = true;
                    }

                    println!(
                        "Executing path to {:?}, cost: {}, remaining movement: {}",
                        event.target_pos, path_preview.path_cost, hero.movement_points
                    );

                    path_preview.clear();
                } else {
                    println!(
                        "Not enough movement points! Need {}, have {}",
                        path_preview.path_cost, hero.movement_points
                    );
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

                    path_preview.set_path(event.target_pos, path, path_cost);

                    if hero.can_move(path_cost) {
                        println!(
                            "Path to {:?} costs {} MP. Click again to execute.",
                            event.target_pos, path_cost
                        );
                    } else {
                        println!(
                            "Path to {:?} costs {} MP (not enough! have {})",
                            event.target_pos, path_cost, hero.movement_points
                        );
                    }
                } else {
                    println!("No path found to {:?}", event.target_pos);
                    path_preview.clear();
                }
            }
            break;
        }
    }
}
