//! One-time generation of pruned test maps for integration tests.
//! Run with: cargo run --bin generate_test_map

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::*;
use moonshine_save::prelude::*;
use rust_imperialism::economy::transport::Rails;
use rust_imperialism::map::province_setup::TestMapConfig;
use rust_imperialism::map::TerrainType;
use rust_imperialism::save::GameSavePlugin;
use rust_imperialism::turn_system::TurnPhase;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;
use std::path::PathBuf;

fn main() {
    println!("Generating pruned test map...");

    let mut app = App::new();

    // Minimal plugins for headless generation
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Initialize game states
    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    // Add save plugin (handles reflection registration)
    app.add_plugins(GameSavePlugin);

    // Add resources normally provided by other plugins
    app.init_resource::<rust_imperialism::civilians::NextCivilianId>();
    app.insert_resource(Rails::default());

    // Insert test config to trigger pruning
    app.insert_resource(TestMapConfig);

    // Track generation state
    app.init_resource::<GenerationState>();

    // Add observer to handle save completion
    app.add_observer(on_save_complete);

    // Add systems for map generation and saving
    app.add_systems(
        Update,
        (
            setup_mock_tilemap,
            rust_imperialism::map::province_setup::generate_provinces_system,
            rust_imperialism::map::province_setup::assign_provinces_to_countries,
            rust_imperialism::map::province_setup::prune_to_test_map,
            mark_tiles_for_save,
            trigger_save,
        )
            .chain()
            .run_if(in_state(AppState::InGame)),
    );

    // Run until save completes
    loop {
        app.update();
        if app.world().resource::<GenerationState>().done {
            break;
        }
    }

    println!("Test map saved successfully!");
}

#[derive(Resource, Default)]
struct GenerationState {
    tilemap_ready: bool,
    pruning_done: bool,
    tiles_marked: bool,
    done: bool,
}

fn setup_mock_tilemap(
    mut commands: Commands,
    tilemap_query: Query<&TileStorage>,
    mut state: ResMut<GenerationState>,
) {
    if state.tilemap_ready || !tilemap_query.is_empty() {
        state.tilemap_ready = true;
        return;
    }

    println!("Creating tilemap...");

    let map_size = TilemapSize { x: 32, y: 32 };
    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        ..default()
                    },
                    TerrainType::Grass,
                ))
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert((
        TilemapGridSize { x: 16.0, y: 16.0 },
        TilemapType::Hexagon(HexCoordSystem::Row),
        map_size,
        tile_storage,
        TilemapTileSize { x: 16.0, y: 16.0 },
    ));

    state.tilemap_ready = true;
}

fn mark_tiles_for_save(
    mut commands: Commands,
    test_config: Option<Res<TestMapConfig>>,
    tiles: Query<Entity, With<TilePos>>,
    mut state: ResMut<GenerationState>,
) {
    // Wait for pruning to complete (TestMapConfig gets removed)
    if test_config.is_some() || state.tiles_marked {
        return;
    }

    if !state.pruning_done {
        println!("Pruning complete, marking tiles for save...");
        state.pruning_done = true;
    }

    // Mark remaining tiles with Save component
    for entity in tiles.iter() {
        commands.entity(entity).insert(Save);
    }

    state.tiles_marked = true;
}

fn trigger_save(
    mut commands: Commands,
    state: Res<GenerationState>,
    mut triggered: Local<bool>,
) {
    if !state.tiles_marked || *triggered {
        return;
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pruned_red_nation.ron");

    println!("Saving to: {:?}", path);

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create fixtures directory");
    }

    // Use moonshine-save directly
    let event = SaveWorld::default_into_file(path)
        .exclude_component::<rust_imperialism::economy::allocation::Allocations>()
        .exclude_component::<rust_imperialism::economy::reservation::ReservationSystem>()
        .include_resource::<rust_imperialism::economy::Calendar>()
        .include_resource::<rust_imperialism::turn_system::TurnCounter>()
        .include_resource::<rust_imperialism::economy::transport::Roads>()
        .include_resource::<rust_imperialism::economy::transport::Rails>()
        .include_resource::<rust_imperialism::civilians::ProspectingKnowledge>()
        .include_resource::<rust_imperialism::civilians::NextCivilianId>()
        .include_resource::<rust_imperialism::map::province_setup::ProvincesGenerated>();

    commands.trigger_save(event);
    *triggered = true;
}

fn on_save_complete(_trigger: On<Saved>, mut state: ResMut<GenerationState>) {
    state.done = true;
}
