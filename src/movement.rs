use crate::pathfinding::PathfindingSystem;
use crate::tile_pos::{HexExt, TilePosExt};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::collections::VecDeque;

/// Component for smooth movement animation between tiles
/// Entities with this component will automatically animate between tile positions
#[derive(Component, Debug, Clone)]
pub struct MovementAnimation {
    pub path: VecDeque<TilePos>,
    pub movement_speed: f32,
    pub is_moving: bool,
    pub target_world_pos: Option<Vec3>,
}

impl Default for MovementAnimation {
    fn default() -> Self {
        Self {
            path: VecDeque::new(),
            movement_speed: 150.0,
            is_moving: false,
            target_world_pos: None,
        }
    }
}

impl MovementAnimation {
    pub fn new(movement_speed: f32) -> Self {
        Self {
            movement_speed,
            ..Default::default()
        }
    }

    pub fn start_movement_to(&mut self, target_pos: Vec3, path: VecDeque<TilePos>) {
        self.target_world_pos = Some(target_pos);
        self.path = path;
        self.is_moving = true;
    }

    pub fn stop_movement(&mut self) {
        self.is_moving = false;
        self.target_world_pos = None;
        self.path.clear();
    }

    pub fn is_idle(&self) -> bool {
        !self.is_moving && self.path.is_empty()
    }
}

/// Component for entities that have movement points and can execute tactical movement
#[derive(Component, Debug, Clone)]
pub struct MovementPoints {
    pub current: u32,
    pub max: u32,
}

impl MovementPoints {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    pub fn can_move(&self, cost: u32) -> bool {
        self.current >= cost
    }

    pub fn consume(&mut self, cost: u32) {
        self.current = self.current.saturating_sub(cost);
    }

    pub fn refresh(&mut self) {
        self.current = self.max;
    }

    pub fn is_exhausted(&self) -> bool {
        self.current == 0
    }
}

impl Default for MovementPoints {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Events for movement requests
#[derive(Event)]
pub struct MoveEntityRequest {
    pub entity: Entity,
    pub target: TilePos,
}

/// Movement capabilities for different entity types
#[derive(Component)]
pub enum MovementType {
    Smart,  // Uses pathfinding (heroes)
    Simple, // Direct neighbor movement (monsters)
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MoveEntityRequest>().add_systems(
            Update,
            (
                movement_animation_system,
                movement_request_system,
                sync_transform_from_tile_pos_system,
            ),
        );
    }
}

/// System to synchronize Transform positions from TilePos for non-moving entities
fn sync_transform_from_tile_pos_system(
    mut entity_query: Query<(&mut Transform, &TilePos, Option<&MovementAnimation>)>,
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

    for (mut transform, tile_pos, movement_anim) in entity_query.iter_mut() {
        // Only sync if entity is not currently animating movement
        if let Some(anim) = movement_anim
            && anim.is_moving {
                continue; // Skip entities that are animating
            }

        // Calculate world position from tile position
        let world_pos = tile_pos.center_in_world(
            tilemap_size,
            grid_size,
            tile_size,
            map_type,
            &TilemapAnchor::Center,
        );

        // Preserve Z coordinate but update X and Y
        transform.translation.x = world_pos.x;
        transform.translation.y = world_pos.y;
    }
}

/// Unified movement animation system
fn movement_animation_system(
    time: Res<Time>,
    mut animated_query: Query<(&mut Transform, &mut MovementAnimation, &mut TilePos)>,
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

    for (mut transform, mut movement_anim, mut tile_pos) in animated_query.iter_mut() {
        if !movement_anim.is_moving {
            continue;
        }

        if let Some(target_pos) = movement_anim.target_world_pos {
            let direction = target_pos - transform.translation;
            let distance = direction.length();

            if distance < 5.0 {
                // Reached current target
                transform.translation = target_pos;
                movement_anim.is_moving = false;
                movement_anim.target_world_pos = None;

                // Update logical position
                if let Some(next_tile) = movement_anim.path.pop_front() {
                    *tile_pos = next_tile;
                }

                // Continue to next waypoint if path has more steps
                if !movement_anim.path.is_empty()
                    && let Some(next_tile) = movement_anim.path.front()
                {
                    let next_world_pos = next_tile
                        .center_in_world(
                            tilemap_size,
                            grid_size,
                            tile_size,
                            map_type,
                            &TilemapAnchor::Center,
                        )
                        .extend(transform.translation.z); // Preserve Z coordinate

                    movement_anim.target_world_pos = Some(next_world_pos);
                    movement_anim.is_moving = true;
                }
            } else {
                // Move towards target
                let move_direction = direction.normalize();
                transform.translation +=
                    move_direction * movement_anim.movement_speed * time.delta_secs();
            }
        }
    }
}

/// System to handle movement requests with unified pathfinding logic
fn movement_request_system(
    mut move_requests: EventReader<MoveEntityRequest>,
    mut entity_query: Query<(
        &mut MovementPoints,
        &mut MovementAnimation,
        &TilePos,
        &MovementType,
    )>,
    tile_query: Query<(&crate::tiles::TileType, &TilePos)>,
    tilemap_query: Query<
        (&TilemapSize, &TileStorage, &TilemapGridSize, &TilemapType),
        With<TilemapGridSize>,
    >,
) {
    let Ok((tilemap_size, tile_storage, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for request in move_requests.read() {
        let Ok((mut points, mut animation, current_pos, movement_type)) =
            entity_query.get_mut(request.entity)
        else {
            continue;
        };

        // Skip if already moving
        if animation.is_moving {
            continue;
        }

        let path = match movement_type {
            MovementType::Smart => {
                // Use pathfinding for smart entities
                PathfindingSystem::find_path_with_combined_query(
                    *current_pos,
                    request.target,
                    tilemap_size,
                    &tile_query,
                    tile_storage,
                )
            }
            MovementType::Simple => {
                // Simple direct movement for monsters
                get_simple_path(*current_pos, request.target, tilemap_size)
            }
        };

        if let Some(path) = path {
            let path_cost = calculate_path_cost(&path, &tile_query, tile_storage);

            if points.can_move(path_cost) {
                points.consume(path_cost);

                if let Some(first_step) = path.first() {
                    let target_world_pos = first_step
                        .center_in_world(
                            tilemap_size,
                            grid_size,
                            &TilemapTileSize { x: 16.0, y: 16.0 },
                            map_type,
                            &TilemapAnchor::Center,
                        )
                        .extend(2.0); // Standard Z level

                    animation.start_movement_to(target_world_pos, path.into());
                }
            }
        }
    }
}

/// Simple pathfinding for monsters (direct neighbor selection)
fn get_simple_path(from: TilePos, to: TilePos, tilemap_size: &TilemapSize) -> Option<Vec<TilePos>> {
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();

    // Get all neighbors of the from position
    let neighbors = from_hex.all_neighbors();

    // Find the neighbor closest to the target
    let next_pos = neighbors
        .into_iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| pos.x < tilemap_size.x && pos.y < tilemap_size.y)
        .min_by_key(|pos| {
            let pos_hex = pos.to_hex();
            pos_hex.distance_to(to_hex)
        })?;

    Some(vec![from, next_pos])
}

/// Calculate path cost using tile movement costs
fn calculate_path_cost(
    path: &[TilePos],
    tile_query: &Query<(&crate::tiles::TileType, &TilePos)>,
    tile_storage: &TileStorage,
) -> u32 {
    let mut total_cost = 0.0;

    for pos in path.iter().skip(1) {
        // Skip starting position
        if let Some(tile_entity) = tile_storage.get(pos)
            && let Ok((tile_type, _)) = tile_query.get(tile_entity)
        {
            if !tile_type.properties.is_passable {
                total_cost += 999.0; // High cost for impassable
            } else {
                total_cost += tile_type.properties.movement_cost;
            }
        } else {
            total_cost += 1.0; // Default cost
        }
    }

    total_cost.ceil() as u32
}
