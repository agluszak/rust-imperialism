//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

use crate::civilians::CivilianPlugin;
use crate::constants::{MAP_SIZE, TERRAIN_SEED, TILE_SIZE};
use crate::debug::DebugPlugins;
use crate::economy::{Calendar, PlaceImprovement, Rails, Roads};
use crate::helpers::camera;
use crate::helpers::picking::TilemapBackend;
use crate::input::{InputPlugin, handle_tile_click};
use crate::terrain_gen::TerrainGenerator;
use crate::tile_pos::TilePosExt;
use crate::transport_rendering::HoveredTile;
use crate::transport_rendering::TransportRenderingPlugin;
use crate::turn_system::TurnSystem;
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;
use crate::ui::components::MapTilemap;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;
use bevy::DefaultPlugins;
use bevy::dev_tools::states::log_transitions;
use bevy::image::ImagePlugin;
use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::prelude::{AppExtStates, Commands, IntoScheduleConfigs, in_state, info};
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_ecs_tilemap::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::{StateInspectorPlugin, WorldInspectorPlugin};

pub mod assets;
pub mod bmp_loader;
pub mod border_rendering;
pub mod city_rendering;
pub mod civilians;
pub mod constants;
pub mod debug;
pub mod economy;
pub mod helpers;
pub mod input;
pub mod province;
pub mod province_gen;
pub mod province_setup;
pub mod terrain_atlas;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;
pub mod transport_rendering;
pub mod turn_system;
pub mod ui;

/// Marker resource to track if tilemap has been created
#[derive(Resource)]
struct TilemapCreated;

/// System that creates the tilemap once the terrain atlas is ready
fn tilemap_startup(
    mut commands: Commands,
    terrain_atlas: Option<Res<terrain_atlas::TerrainAtlas>>,
    tilemap_created: Option<Res<TilemapCreated>>,
) {
    // Skip if tilemap already created
    if tilemap_created.is_some() {
        return;
    }

    // Wait for the terrain atlas to be built
    let Some(atlas) = terrain_atlas else {
        return;
    };

    if !atlas.ready {
        return;
    }

    info!("Terrain atlas ready, creating tilemap...");

    let map_size = TilemapSize {
        x: MAP_SIZE,
        y: MAP_SIZE,
    };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    // Create terrain generator with a fixed seed for consistent worlds
    let terrain_gen = TerrainGenerator::new(TERRAIN_SEED);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };

            // Generate terrain using noise functions
            let terrain_type = terrain_gen.generate_terrain(x, y, map_size.x, map_size.y);
            let texture_index = terrain_type.get_texture_index();

            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(texture_index),
                        ..default()
                    },
                    terrain_type, // Add the terrain type component
                ))
                .observe(handle_tile_click)
                .observe(handle_tile_hover)
                .observe(handle_tile_out)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    // Log some sample tile positions to debug rendering
    let sample_tiles = vec![
        TilePos { x: 0, y: 0 },
        TilePos { x: 15, y: 15 },
        TilePos { x: 31, y: 31 },
    ];
    for tile in sample_tiles {
        let world_pos = tile.to_world_pos();
        info!(
            "Tile ({}, {}) world pos: ({:.1}, {:.1})",
            tile.x, tile.y, world_pos.x, world_pos.y
        );
    }

    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = constants::get_hex_grid_size();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    commands.entity(tilemap_entity).insert((
        TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(atlas.texture.clone()),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..Default::default()
        },
        MapTilemap, // Marker to control visibility
    ));

    // Mark tilemap as created
    commands.insert_resource(TilemapCreated);

    info!("Tilemap created successfully!");
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

/// Track when mouse enters a tile
fn handle_tile_hover(
    trigger: On<Pointer<Over>>,
    tile_positions: Query<&TilePos>,
    mut hovered_tile: ResMut<HoveredTile>,
) {
    if let Ok(tile_pos) = tile_positions.get(trigger.entity) {
        hovered_tile.0 = Some(*tile_pos);
    }
}

/// Track when mouse leaves a tile
fn handle_tile_out(_trigger: On<Pointer<Out>>, mut hovered_tile: ResMut<HoveredTile>) {
    hovered_tile.0 = None;
}


pub fn app() -> App {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        bmp_loader::ImperialismBmpLoaderPlugin,
    ))
    .insert_state(AppState::MainMenu)
    .add_sub_state::<GameMode>()
    .add_systems(Update, log_transitions::<AppState>)
    .add_systems(Update, log_transitions::<GameMode>)
    // Root app state: start in Main Menu; also set up in-game mode state (Map)
    .insert_resource(Calendar::default())
    .insert_resource(Roads::default())
    .insert_resource(Rails::default())
    .add_message::<PlaceImprovement>()
    .add_systems(Startup, (setup_camera,))
    // Start loading terrain atlas at startup
    .add_systems(Startup, terrain_atlas::start_terrain_atlas_loading)
    // Build atlas when tiles are loaded
    .add_systems(Update, terrain_atlas::build_terrain_atlas_when_ready)
    // Nations are now created during province assignment (see province_setup.rs)
    // Tilemap startup runs in Update and waits for atlas to be ready
    .add_systems(Update, tilemap_startup.run_if(in_state(AppState::InGame)))
    // Province generation runs after tilemap is created
    .add_systems(Update, (
        province_setup::generate_provinces_system,
        province_setup::assign_provinces_to_countries
            .after(province_setup::generate_provinces_system),
    ).run_if(in_state(AppState::InGame)))
    // Economy systems
    .add_systems(
        Update,
        (
            economy::transport::apply_improvements,
            economy::transport::compute_rail_connectivity
                .after(economy::transport::apply_improvements),
            economy::production::run_production,
            // Advance rail construction at the start of each player turn
            economy::transport::advance_rail_construction
                .run_if(resource_changed::<TurnSystem>)
                .run_if(|turn_system: Res<TurnSystem>| {
                    turn_system.phase == turn_system::TurnPhase::PlayerTurn
                }),
            // Reset civilian actions at the start of each player turn
            civilians::reset_civilian_actions
                .run_if(resource_changed::<TurnSystem>)
                .run_if(|turn_system: Res<TurnSystem>| {
                    turn_system.phase == turn_system::TurnPhase::PlayerTurn
                }),
            // Advance civilian jobs at the start of each player turn
            civilians::advance_civilian_jobs
                .run_if(resource_changed::<TurnSystem>)
                .run_if(|turn_system: Res<TurnSystem>| {
                    turn_system.phase == turn_system::TurnPhase::PlayerTurn
                }),
        )
            .run_if(in_state(AppState::InGame)),
    )
    .add_systems(
        Update,
        camera::movement
            .after(ui::handle_mouse_wheel_scroll)
            .run_if(in_state(GameMode::Map)),
    )
    .add_plugins((
        TilemapPlugin,
        TilemapBackend,
        // Game plugins (strategy baseline)
        TurnSystemPlugin,
        GameUIPlugin,
        InputPlugin,
        TransportRenderingPlugin,
        CivilianPlugin,
        border_rendering::BorderRenderingPlugin,
        city_rendering::CityRenderingPlugin,
    ))
    .add_plugins(DebugPlugins)
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::new())
    .add_plugins(StateInspectorPlugin::<AppState>::new())
    .add_plugins(StateInspectorPlugin::<GameMode>::new());

    app
}

#[cfg(test)]
pub mod test_utils;
