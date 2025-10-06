//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

use crate::civilians::{Civilian, CivilianKind, CivilianPlugin};
use crate::constants::{MAP_SIZE, TERRAIN_SEED, TILE_SIZE};
use crate::debug::DebugPlugins;
use crate::economy::{
    Building, Calendar, Capital, Good, Name, NationId, PlaceImprovement, PlayerNation, Rails,
    Roads, Stockpile, Technologies, Treasury,
};
use crate::helpers::camera;
use crate::helpers::picking::TilemapBackend;
use crate::input::{InputPlugin, handle_tile_click};
use crate::terrain_gen::TerrainGenerator;
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
use bevy::prelude::{AppExtStates, Commands, IntoScheduleConfigs, OnEnter, in_state, info};
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_ecs_tilemap::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::{StateInspectorPlugin, WorldInspectorPlugin};

pub mod civilians;
pub mod constants;
pub mod debug;
pub mod economy;
pub mod helpers;
pub mod input;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;
pub mod transport_rendering;
pub mod turn_system;
pub mod ui;

fn tilemap_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Asset by Kenney
    let texture_handle: Handle<Image> = asset_server.load("colored_packed.png");
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
                .observe(handle_tile_hover)
                .observe(handle_tile_out)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    commands.entity(tilemap_entity).insert((
        TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..Default::default()
        },
        MapTilemap, // Marker to control visibility
    ));
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

// Spawn initial nations when entering InGame
fn setup_nations(mut commands: Commands) {
    // Player nation (capital at center of map)
    let mut player_stock = Stockpile::default();
    player_stock.add(Good::Wool, 10);
    player_stock.add(Good::Cotton, 10);

    let player_capital = TilePos {
        x: MAP_SIZE / 2,
        y: MAP_SIZE / 2,
    };

    let player_entity = commands
        .spawn((
            NationId(1),
            Name("Player".to_string()),
            Capital(player_capital),
            Treasury::default(),
            player_stock,
            Building::textile_mill(4),
            Technologies::default(), // Start with no technologies
        ))
        .id();

    // Spawn an Engineer unit for the player near their capital
    let engineer_start = TilePos {
        x: MAP_SIZE / 2 + 2,
        y: MAP_SIZE / 2 + 1,
    };
    info!(
        "Spawning Engineer at tile position: ({}, {})",
        engineer_start.x, engineer_start.y
    );

    let _engineer_entity = commands
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: engineer_start,
            owner: player_entity,
            selected: false,
            has_moved: false,
        })
        .id();

    // Simple AI nation (capital in a different location)
    let ai_capital = TilePos { x: 5, y: 5 };

    commands.spawn((
        NationId(2),
        Name("Rivalia".to_string()),
        Capital(ai_capital),
        Treasury(40_000),
        Stockpile::default(),
        Technologies::default(),
    ));

    // Set the player's nation reference for UI/controllers
    commands.insert_resource(PlayerNation(player_entity));
}

pub fn app() -> App {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest()),))
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
        // Bootstrap nations and spawn map when starting a new game
        .add_systems(OnEnter(AppState::InGame), (setup_nations, tilemap_startup))
        // Economy systems
        .add_systems(
            Update,
            (
                economy::transport::apply_improvements,
                crate::economy::transport::compute_rail_connectivity
                    .after(crate::economy::transport::apply_improvements),
                crate::economy::production::run_production,
                // Advance rail construction at the start of each player turn
                crate::economy::transport::advance_rail_construction
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == crate::turn_system::TurnPhase::PlayerTurn
                    }),
                // Reset civilian actions at the start of each player turn
                crate::civilians::reset_civilian_actions
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == crate::turn_system::TurnPhase::PlayerTurn
                    }),
                // Advance civilian jobs at the start of each player turn
                crate::civilians::advance_civilian_jobs
                    .run_if(resource_changed::<TurnSystem>)
                    .run_if(|turn_system: Res<TurnSystem>| {
                        turn_system.phase == crate::turn_system::TurnPhase::PlayerTurn
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
