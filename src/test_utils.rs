//! Testing utilities for Rust Imperialism
//!
//! This module provides helper functions and fixtures for unit testing
//! game systems in isolation. ECS makes testing particularly clean since
//! we can easily mock entities and components.

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::health::{Combat, Health};
use crate::hero::{Hero, HeroPathPreview};
use crate::monster::{Monster, MonsterBehavior};
use crate::movement::{ActionPoints, MovementAnimation, MovementType};
use crate::tiles::{TerrainType, TileType};
use crate::turn_system::{TurnPhase, TurnSystem};
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
            let tile_type = TileType::terrain(TerrainType::Grass);

            let tile_entity = world
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(0),
                        ..default()
                    },
                    tile_type,
                ))
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

/// Creates a test hero entity with default stats
pub fn create_test_hero(world: &mut World, position: TilePos) -> Entity {
    world
        .spawn((
            Hero {
                name: "Test Hero".to_string(),
                is_selected: false,
                kills: 0,
            },
            Health::new(100),
            Combat::new(25),
            ActionPoints::new(6),
            MovementAnimation::new(200.0),
            MovementType::Smart,
            HeroPathPreview::default(),
            position,
            Transform::default(),
        ))
        .id()
}

/// Creates a test monster entity with specified behavior
pub fn create_test_monster(
    world: &mut World,
    position: TilePos,
    behavior: MonsterBehavior,
    health: u32,
) -> Entity {
    world
        .spawn((
            Monster {
                name: "Test Monster".to_string(),
                sight_range: 5,
                behavior,
            },
            Health::new(health),
            Combat::new(10),
            ActionPoints::new(4),
            MovementAnimation::new(150.0),
            MovementType::Simple,
            position,
            Transform::default(),
        ))
        .id()
}

/// Creates a test tile with the specified terrain type at the given position
pub fn create_test_tile(
    world: &mut World,
    position: TilePos,
    terrain: TerrainType,
    tilemap_entity: Entity,
    tile_storage: &mut TileStorage,
) -> Entity {
    let tile_type = TileType::terrain(terrain);
    let tile_entity = world
        .spawn((
            TileBundle {
                position,
                tilemap_id: TilemapId(tilemap_entity),
                texture_index: TileTextureIndex(tile_type.get_texture_index()),
                ..default()
            },
            tile_type,
        ))
        .id();

    tile_storage.set(&position, tile_entity);
    tile_entity
}

// Macros removed - use regular test functions with explicit setup instead

/// Advances the turn system by the specified number of phases
pub fn advance_turns(world: &mut World, phases: usize) {
    for _ in 0..phases {
        world.resource_mut::<TurnSystem>().advance_turn();
    }
}

/// Sets up a combat scenario with a hero and monster
pub fn setup_combat_scenario(
    world: &mut World,
    hero_pos: TilePos,
    monster_pos: TilePos,
) -> (Entity, Entity) {
    let hero = create_test_hero(world, hero_pos);
    let monster = create_test_monster(world, monster_pos, MonsterBehavior::Aggressive, 50);
    (hero, monster)
}

/// Asserts that two tile positions are adjacent (distance = 1)
pub fn assert_adjacent(pos1: TilePos, pos2: TilePos) {
    use crate::tile_pos::TilePosExt;
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
