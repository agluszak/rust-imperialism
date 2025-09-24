//! Rust Imperialism - A hexagonal tile-based strategy game

use bevy::prelude::*;
use bevy_ecs_tilemap::map::HexCoordSystem;
use bevy_ecs_tilemap::prelude::*;

// Import all game modules
mod combat;
mod health;
mod helpers;
mod hero;
mod input;
mod monster;
mod movement;
mod pathfinding;
mod terrain_gen;
mod tile_pos;
mod tiles;
mod turn_system;
mod ui;

use crate::combat::CombatPlugin;
use crate::health::{Combat, Health};
use crate::helpers::{camera, picking::TilemapBackend};
use crate::hero::{Hero, HeroPathPreview, HeroPlugin, HeroSprite};
use crate::input::{InputPlugin, handle_tile_click};
use crate::monster::MonsterPlugin;
use crate::movement::{ActionPoints, MovementAnimation, MovementPlugin, MovementType};
use crate::terrain_gen::TerrainGenerator;
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;

/// Generate a realistic terrain map using noise functions
fn tilemap_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Asset by Kenney
    let texture_handle: Handle<Image> = asset_server.load("colored_packed.png");
    let map_size = TilemapSize { x: 32, y: 32 }; // Larger map for better noise patterns

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    // Create terrain generator with a fixed seed for consistent worlds
    let terrain_gen = TerrainGenerator::new(12345);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };

            // Generate terrain using noise functions
            let tile_type = terrain_gen.generate_terrain(x, y, map_size.x, map_size.y);
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
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        anchor: TilemapAnchor::Center,
        ..Default::default()
    });
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            TilemapPlugin,
            TilemapBackend,
            // Game plugins
            MovementPlugin,
            HeroPlugin,
            MonsterPlugin,
            TurnSystemPlugin,
            GameUIPlugin,
            InputPlugin,
            CombatPlugin,
        ))
        .add_systems(
            Startup,
            (
                tilemap_startup,
                setup_camera,
                spawn_hero.after(tilemap_startup),
            ),
        )
        .add_systems(
            Update,
            camera::movement.after(ui::handle_mouse_wheel_scroll),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn spawn_hero(
    mut commands: Commands,
    tilemap_query: Query<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
        ),
        With<TilemapGridSize>,
    >,
) {
    let Ok((tilemap_size, grid_size, tile_size, map_type)) = tilemap_query.single() else {
        return;
    };

    // Spawn hero at center position (16, 16) for 32x32 map
    let hero_pos = TilePos { x: 16, y: 16 };
    let hero_world_pos = hero_pos
        .center_in_world(
            tilemap_size,
            grid_size,
            tile_size,
            map_type,
            &TilemapAnchor::Center,
        )
        .extend(2.0); // Place hero well above tiles

    commands.spawn((
        Hero {
            name: "Player Hero".to_string(),
            is_selected: false,
            kills: 0,
        },
        ActionPoints::new(6),          // 6 action points
        MovementAnimation::new(200.0), // Hero movement speed
        MovementType::Smart,           // Heroes use pathfinding
        HeroPathPreview::default(),
        hero_pos,
        Health::new(100),
        Combat::new(25),
        HeroSprite,
        Sprite {
            color: Color::srgb(0.0, 0.0, 1.0), // Blue square
            custom_size: Some(Vec2::new(16.0, 16.0)),
            ..default()
        },
        Transform::from_translation(hero_world_pos),
    ));
}
