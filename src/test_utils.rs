//! Testing utilities for Rust Imperialism
//!
//! This module provides helper functions and fixtures for unit testing
//! game systems in isolation. ECS makes testing particularly clean since
//! we can easily mock entities and components.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::map::tiles::TerrainType;
use crate::turn_system::TurnSystem;
use crate::ui::state::UIState;

/// Creates a minimal ECS world for testing with commonly needed resources
pub fn create_test_world() -> World {
    let mut world = World::new();

    // Add common resources that most tests need
    world.insert_resource(Time::<()>::default());
    world.insert_resource(TurnSystem::default());
    world.insert_resource(UIState::default());

    world
}

/// Creates a test tilemap with the specified dimensions
/// Returns the tilemap entity and tile storage
pub fn create_test_tilemap(world: &mut World, width: u32, height: u32) -> (Entity, TileStorage) {
    let map_size = TilemapSize {
        x: width,
        y: height,
    };
    let tilemap_entity = world.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    // Create tiles with grass terrain by default
    for x in 0..width {
        for y in 0..height {
            let tile_pos = TilePos { x, y };

            let tile_entity = world
                .spawn((TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(0),
                    ..default()
                },))
                .id();

            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    // Configure the tilemap entity
    world.entity_mut(tilemap_entity).insert((
        TilemapGridSize { x: 16.0, y: 16.0 },
        TilemapType::Hexagon(HexCoordSystem::Row),
        map_size,
        tile_storage.clone(),
        TilemapTexture::Single(Handle::default()),
        TilemapTileSize { x: 16.0, y: 16.0 },
        TilemapAnchor::Center,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    (tilemap_entity, tile_storage)
}

/// Creates a test tile with the specified terrain type at the given position
pub fn create_test_tile(
    world: &mut World,
    position: TilePos,
    terrain: TerrainType,
    tilemap_entity: Entity,
    tile_storage: &mut TileStorage,
) -> Entity {
    let tile_entity = world
        .spawn((
            TileBundle {
                position,
                tilemap_id: TilemapId(tilemap_entity),
                texture_index: TileTextureIndex(terrain.get_texture_index()),
                ..default()
            },
            terrain,
        ))
        .id();

    tile_storage.set(&position, tile_entity);
    tile_entity
}

// Macros removed - use regular test functions with explicit setup instead

/// Advances the turn phase by cycling through phases.
///
/// Note: This is a test utility that directly manipulates the legacy TurnSystem.
/// In production code, use state transitions via NextState<TurnPhase>.
pub fn advance_turns(world: &mut World, phases: usize) {
    use crate::turn_system::TurnPhase;
    for _ in 0..phases {
        let mut turn_system = world.resource_mut::<TurnSystem>();
        turn_system.phase = match turn_system.phase {
            TurnPhase::PlayerTurn => TurnPhase::Processing,
            TurnPhase::Processing => TurnPhase::EnemyTurn,
            TurnPhase::EnemyTurn => {
                turn_system.current_turn += 1;
                TurnPhase::PlayerTurn
            }
        };
    }
}

/// Asserts that two tile positions are adjacent (distance = 1)
pub fn assert_adjacent(pos1: TilePos, pos2: TilePos) {
    use crate::map::tile_pos::TilePosExt;
    let hex1 = pos1.to_hex();
    let hex2 = pos2.to_hex();
    let distance = hex1.distance_to(hex2);
    assert_eq!(
        distance, 1,
        "Positions {:?} and {:?} are not adjacent (distance: {})",
        pos1, pos2, distance
    );
}

/// Asserts that a path is valid (each step is adjacent to the previous)
pub fn assert_valid_path(path: &[TilePos]) {
    if path.len() < 2 {
        return; // Single position or empty path is valid
    }

    for window in path.windows(2) {
        assert_adjacent(window[0], window[1]);
    }
}

/// Mock event writer that collects events for inspection in tests
#[derive(Default)]
pub struct MockEventWriter<T: Message> {
    pub events: Vec<T>,
}

impl<T: Message> MockEventWriter<T> {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn write(&mut self, event: T) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
