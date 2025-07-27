//! Example of tiles receiving pick events
//! Click on a tile to change its texture.
//!
//! Camera Controls:
//! - WASD: Move camera
//! - Z: Zoom out (keyboard)
//! - X: Zoom in (keyboard)
//! - Mouse wheel: Zoom in/out

use bevy::prelude::*;
use bevy_ecs_tilemap::map::HexCoordSystem;
use bevy_ecs_tilemap::prelude::*;

mod combat;
mod health;
mod helpers;
mod hero;
mod input;
mod monster;
mod pathfinding;
mod tile_pos;
mod tiles;
mod turn_system;
mod ui;

use crate::combat::CombatPlugin;
use crate::health::{Combat, Health};
use crate::helpers::camera;
use crate::helpers::picking::TilemapBackend;
use crate::hero::{Hero, HeroMovement, HeroPathPreview, HeroPlugin, HeroSprite, PathPreviewMarker};
use crate::input::{InputPlugin, handle_tile_click};
use crate::monster::MonsterPlugin;
use crate::tiles::{TerrainType, TileType};
use crate::turn_system::{TurnSystem, TurnSystemPlugin};
use crate::ui::GameUIPlugin;

/// mostly the same as the `basic` example from `bevy_ecs_tilemap`
fn tilemap_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Print controls to console
    println!("=== Game Controls ===");
    println!("WASD: Move camera");
    println!("Z: Zoom out (keyboard)");
    println!("X: Zoom in (keyboard)");
    println!("Mouse wheel: Zoom in/out");
    println!("Left click: Select hero or move hero");
    println!("Right click: Cycle terrain types");
    println!("Space: End turn");
    println!("=====================");

    // Asset by Kenney
    let texture_handle: Handle<Image> = asset_server.load("colored_packed.png");
    let map_size = TilemapSize { x: 20, y: 20 };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };

            // Create different terrain types based on position for variety
            let tile_type = if x < 5 {
                TileType::terrain(TerrainType::Water)
            } else if x > 15 {
                TileType::terrain(TerrainType::Mountain)
            } else if y < 5 {
                TileType::terrain(TerrainType::Forest)
            } else if y > 15 {
                TileType::terrain(TerrainType::Desert)
            } else {
                TileType::terrain(TerrainType::Grass)
            };

            let texture_index = tile_type.get_texture_index();

            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(texture_index),
                        ..default()
                    },
                    tile_type, // Add the tile type component
                ))
                .observe(handle_tile_click)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16., y: 16. };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle.clone()),
        tile_size,
        anchor: TilemapAnchor::Center,
        ..Default::default()
    });

    // Spawn hero at starting position
    let hero_pos = TilePos { x: 10, y: 10 };
    let hero_world_pos = hero_pos.center_in_world(
        &map_size,
        &grid_size,
        &tile_size,
        &map_type,
        &TilemapAnchor::Center,
    );

    commands.spawn((
        Hero::new("Player Hero".to_string(), 3),
        Health::new(10),
        Combat::new(3),
        HeroMovement::default(),
        HeroPathPreview::default(),
        hero_pos,
        HeroSprite,
        Sprite {
            color: Color::srgb(0.0, 0.0, 1.0), // Blue color for hero
            custom_size: Some(Vec2::new(12.0, 12.0)),
            ..default()
        },
        Transform::from_translation(hero_world_pos.extend(1.0)),
    ));
}

// The tile click handler is now much simpler - just dispatch to input system

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            TilemapPlugin,
            // The additional backend to check events against the tiles
            TilemapBackend,
            // Game systems
            TurnSystemPlugin,
            HeroPlugin,
            GameUIPlugin,
            MonsterPlugin,
            CombatPlugin,
            InputPlugin,
        ))
        .add_systems(
            Startup,
            (tilemap_startup, |mut commands: Commands| {
                commands.spawn((
                    Camera2d,
                    Projection::Orthographic(OrthographicProjection {
                        scale: 0.5,
                        ..OrthographicProjection::default_2d()
                    }),
                ));
            }),
        )
        .add_systems(
            Update,
            (
                camera::movement,
                hero_turn_refresh,
                update_hero_position,
                clear_path_preview_on_turn_change,
            ),
        )
        .run();
}

// System to refresh hero movement points at start of turn
fn hero_turn_refresh(mut hero_query: Query<&mut Hero>, turn_system: Res<TurnSystem>) {
    if turn_system.is_changed() && turn_system.is_player_turn() {
        for mut hero in hero_query.iter_mut() {
            hero.refresh_movement();
        }
    }
}

// System to update hero tile position when movement is complete
fn update_hero_position(
    mut hero_query: Query<(&mut TilePos, &HeroMovement, &Transform), With<Hero>>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
) {
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for (mut tile_pos, movement, transform) in hero_query.iter_mut() {
        if !movement.is_moving && movement.path.is_empty() {
            // Update tile position based on world position
            if let Some(new_pos) = TilePos::from_world_pos(
                &transform.translation.xy(),
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            ) && *tile_pos != new_pos
            {
                *tile_pos = new_pos;
            }
        }
    }
}

// System to clear path preview markers when turn changes
fn clear_path_preview_on_turn_change(
    mut commands: Commands,
    turn_system: Res<TurnSystem>,
    preview_markers: Query<Entity, With<PathPreviewMarker>>,
    mut hero_query: Query<&mut HeroPathPreview, With<Hero>>,
) {
    if turn_system.is_changed() {
        // Clear all path preview markers
        for entity in preview_markers.iter() {
            commands.entity(entity).despawn();
        }

        // Clear all hero path previews
        for mut path_preview in hero_query.iter_mut() {
            path_preview.clear();
        }
    }
}
