use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::window::{Window, WindowPlugin, WindowResolution};
use bevy_ecs_tilemap::prelude::*;
use moonshine_save::prelude::*;
use rust_imperialism::constants::{MAP_SIZE, TILE_SIZE, get_hex_grid_size};
use rust_imperialism::map::rendering::terrain_atlas::TerrainAtlas;
use rust_imperialism::ui::components::MapTilemap;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::{LogicPlugins, MapRenderingPlugins};
use image::GenericImageView;

fn main() {
    if std::env::var("CI").is_ok() {
        // In CI, we expect a proper setup (xvfb), so we let it fail if missing.
        // But for local headless dev or this sandbox, we might want to skip or warn.
        // Actually, let's just check for display variables.
    }

    if std::env::var("DISPLAY").is_err()
        && std::env::var("WAYLAND_DISPLAY").is_err()
        && std::env::var("WAYLAND_SOCKET").is_err()
    {
        println!("Skipping screenshot test: no display available (DISPLAY/WAYLAND_DISPLAY/WAYLAND_SOCKET not set).");
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let reference_path = manifest_dir
        .join("screenshots")
        .join("pruned_red_nation.png");

    // Use a temp file for the new screenshot
    let output_path = std::env::temp_dir().join("test_pruned_red_nation.png");
    // Ensure we clean up previous runs
    if output_path.exists() {
        let _ = std::fs::remove_file(&output_path);
    }

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Fixture Map Screenshot Test".to_string(),
                    resolution: WindowResolution::new(1600, 1200),
                    resizable: false,
                    // Hide the window to avoid annoying popups during testing,
                    // though it still requires a windowing context.
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
    );

    // Use Logic and Map Rendering groups
    app.add_plugins((LogicPlugins, MapRenderingPlugins));

    app.init_resource::<RenderState>();
    app.insert_resource(ScreenshotPath(output_path.clone()));

    // Force InGame state to trigger plugin systems
    app.insert_state(AppState::InGame);

    app.add_observer(on_loaded);
    app.add_observer(exit_after_capture);

    app.add_systems(Startup, request_fixture_load);
    app.add_systems(
        Update,
        (clear_loaded_tilemap_refs, build_tilemap_from_fixture).chain(),
    );
    app.add_systems(Update, (fit_camera_to_map, request_screenshot));

    app.run();

    // Comparison logic
    assert!(output_path.exists(), "Screenshot was not created");

    let ref_img = image::open(&reference_path).expect("Failed to open reference image");
    let new_img = image::open(&output_path).expect("Failed to open new image");

    assert_eq!(ref_img.dimensions(), new_img.dimensions(), "Image dimensions mismatch");

    let diff = diff_images(&ref_img, &new_img);
    let total_pixels = ref_img.width() as u64 * ref_img.height() as u64;
    // Allow 0.1% difference to account for minor rendering variations across platforms
    let tolerance = total_pixels / 1000;

    assert!(
        diff <= tolerance,
        "Images differ by {} pixels, which exceeds tolerance of {} pixels (0.1%)",
        diff,
        tolerance
    );

    // Clean up if successful
    let _ = std::fs::remove_file(output_path);
}

fn diff_images(img1: &image::DynamicImage, img2: &image::DynamicImage) -> u64 {
    let mut diff_pixels = 0;

    for y in 0..img1.height() {
        for x in 0..img1.width() {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);
            if p1 != p2 {
                diff_pixels += 1;
            }
        }
    }
    diff_pixels
}

#[derive(Resource, Default)]
struct RenderState {
    loaded: bool,
    cleared: bool,
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

fn clear_loaded_tilemap_refs(
    mut commands: Commands,
    mut state: ResMut<RenderState>,
    tilemaps: Query<Entity, With<TileStorage>>,
    tiles: Query<Entity, With<TilemapId>>,
) {
    if !state.loaded || state.cleared {
        return;
    }

    for entity in tilemaps.iter() {
        commands.entity(entity).despawn();
    }
    for entity in tiles.iter() {
        commands.entity(entity).remove::<TilemapId>();
    }

    state.cleared = true;
}

fn build_tilemap_from_fixture(
    mut commands: Commands,
    mut state: ResMut<RenderState>,
    atlas: Option<Res<TerrainAtlas>>,
    tiles: Query<(Entity, &TilePos), With<TileTextureIndex>>,
) {
    if state.tilemap_ready || !state.loaded || !state.cleared {
        return;
    }

    let Some(atlas) = atlas else {
        return;
    };
    if !atlas.ready {
        return;
    }

    let mut tile_entries = Vec::new();
    for (entity, pos) in tiles.iter() {
        tile_entries.push((entity, *pos));
    }

    if tile_entries.is_empty() {
        return;
    }

    let map_size = TilemapSize {
        x: MAP_SIZE,
        y: MAP_SIZE,
    };

    let mut tile_storage = TileStorage::empty(map_size);
    for (entity, pos) in tile_entries.iter() {
        tile_storage.set(pos, *entity);
    }

    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = get_hex_grid_size();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    let tilemap_entity = commands
        .spawn((
            TilemapBundle {
                grid_size,
                map_type,
                size: map_size,
                storage: tile_storage,
                texture: TilemapTexture::Single(atlas.texture.clone()),
                tile_size,
                anchor: TilemapAnchor::Center,
                ..default()
            },
            MapTilemap,
        ))
        .id();

    for (entity, pos) in tile_entries {
        commands.entity(entity).insert(pos);
        commands.entity(entity).insert(TilemapId(tilemap_entity));
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
        .observe(save_to_disk(path.0.clone()));

    state.screenshot_requested = true;
}

fn exit_after_capture(_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
