//! Render the pruned red nation fixture and save a screenshot to disk.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::window::{Window, WindowPlugin, WindowResolution};
use bevy_ecs_tilemap::prelude::*;
use moonshine_save::prelude::*;
use rust_imperialism::bmp_loader::ImperialismBmpLoaderPlugin;
use rust_imperialism::map::MapGenerationConfig;
use rust_imperialism::plugins::{LogicPlugins, MapRenderingPlugins};
use rust_imperialism::ui::components::MapTilemap;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;

fn main() {
    let screenshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("screenshots")
        .join("pruned_red_nation.png");

    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Fixture Map Screenshot".to_string(),
                    resolution: WindowResolution::new(1600, 1200),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        ImperialismBmpLoaderPlugin,
        TilemapPlugin,
        LogicPlugins {
            map_generation: MapGenerationConfig { enabled: false },
        },
        MapRenderingPlugins,
    ));

    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    app.init_resource::<RenderState>();
    app.insert_resource(ScreenshotPath(screenshot_path));

    app.add_observer(on_loaded);

    app.add_systems(Startup, (request_fixture_load, setup_camera));
    app.add_systems(Update, mark_tilemap_ready);
    app.add_systems(Update, (fit_camera_to_map, request_screenshot));

    app.run();
}

#[derive(Resource, Default)]
struct RenderState {
    loaded: bool,
    tilemap_ready: bool,
    camera_fitted: bool,
    frames_since_ready: u32,
    screenshot_requested: bool,
}

#[derive(Resource, Debug, Clone)]
struct ScreenshotPath(PathBuf);

fn request_fixture_load(mut commands: Commands, path: Res<ScreenshotPath>) {
    if let Some(parent) = path.0.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create screenshot directory");
    }

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pruned_red_nation.ron");

    commands.trigger_load(LoadWorld::default_from_file(fixture_path));
}

fn on_loaded(_: On<Loaded>, mut state: ResMut<RenderState>) {
    state.loaded = true;
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn mark_tilemap_ready(tilemaps: Query<Entity, With<MapTilemap>>, mut state: ResMut<RenderState>) {
    if state.tilemap_ready || !state.loaded {
        return;
    }

    if tilemaps.is_empty() {
        return;
    }

    state.tilemap_ready = true;
}

fn fit_camera_to_map(
    windows: Query<&Window>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    tilemap_query: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
    tiles: Query<&TilePos, With<TileTextureIndex>>,
    mut state: ResMut<RenderState>,
) {
    if !state.tilemap_ready || state.camera_fitted {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((map_size, grid_size, tile_size, map_type, anchor)) = tilemap_query.single() else {
        return;
    };
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };

    let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);

    for pos in tiles.iter() {
        let world = pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);
        min.x = min.x.min(world.x);
        min.y = min.y.min(world.y);
        max.x = max.x.max(world.x);
        max.y = max.y.max(world.y);
    }

    let center = (min + max) * 0.5;
    let extent = max - min;
    let scale_x = extent.x / window.resolution.width();
    let scale_y = extent.y / window.resolution.height();
    let scale = scale_x.max(scale_y) * 1.1;

    transform.translation.x = center.x;
    transform.translation.y = center.y;
    transform.translation.z = 999.0;

    if let Projection::Orthographic(ortho) = &mut *projection {
        ortho.scale = scale.max(0.1);
    }

    state.camera_fitted = true;
}

fn request_screenshot(
    mut commands: Commands,
    mut state: ResMut<RenderState>,
    path: Res<ScreenshotPath>,
) {
    if !state.camera_fitted || state.screenshot_requested {
        return;
    }

    state.frames_since_ready += 1;
    if state.frames_since_ready < 20 {
        return;
    }

    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.0.clone()))
        .observe(exit_after_capture);

    state.screenshot_requested = true;
}

fn exit_after_capture(_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
