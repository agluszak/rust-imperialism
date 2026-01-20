use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapId, TilemapSize};
use moonshine_save::prelude::*;
use rust_imperialism::map::MapGenerationConfig;
use rust_imperialism::plugins::LogicPlugins;
use rust_imperialism::turn_system::TurnPhase;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;

/// Helper function to transition between turn phases in tests
/// Encapsulates the double-update pattern needed for state transitions
pub fn transition_to_phase(app: &mut bevy::app::App, phase: TurnPhase) {
    app.world_mut()
        .resource_mut::<NextState<TurnPhase>>()
        .set(phase);
    app.update(); // Apply state transition
    app.update(); // Run systems in the new phase
}

/// Get the path to a test fixture file
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Filename for the pruned Red nation test map
pub const PRUNED_RED_NATION_MAP: &str = "pruned_red_nation.ron";

/// Resource to track if a fixture load has completed
#[derive(Resource, Default)]
struct FixtureLoadCompleted(bool);

/// Observer that marks when load is complete
fn on_fixture_loaded(_: On<Loaded>, mut completed: ResMut<FixtureLoadCompleted>) {
    completed.0 = true;
}

/// Creates a test app configured for loading fixtures
pub fn create_fixture_test_app() -> bevy::app::App {
    let mut app = bevy::app::App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    app.add_plugins(LogicPlugins {
        map_generation: MapGenerationConfig { enabled: false },
    });

    // Add load completion tracking
    app.init_resource::<FixtureLoadCompleted>();
    app.add_observer(on_fixture_loaded);

    app
}

/// Creates a test app configured for turn-based simulations with AI/economy logic.
pub fn create_fixture_simulation_app() -> bevy::app::App {
    let mut app = bevy::app::App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    app.add_plugins(LogicPlugins {
        map_generation: MapGenerationConfig { enabled: false },
    });

    app.init_resource::<FixtureLoadCompleted>();
    app.add_observer(on_fixture_loaded);

    app
}

/// Rebuilds TileStorage from loaded tiles and spawns a tilemap entity.
pub fn rebuild_tile_storage(app: &mut bevy::app::App) -> Entity {
    let (tiles, map_size, existing_tilemaps) = {
        let world = app.world_mut();

        let mut tile_query = world.query_filtered::<(Entity, &TilePos), With<TilemapId>>();
        let mut tiles = Vec::new();
        let mut max_x = 0;
        let mut max_y = 0;

        for (entity, pos) in tile_query.iter(world) {
            tiles.push((entity, *pos));
            max_x = max_x.max(pos.x);
            max_y = max_y.max(pos.y);
        }

        if tiles.is_empty() {
            panic!("No tiles loaded; cannot rebuild TileStorage.");
        }

        let mut tilemap_query = world.query_filtered::<Entity, With<TileStorage>>();
        let existing_tilemaps: Vec<Entity> = tilemap_query.iter(world).collect();

        let map_size = TilemapSize {
            x: max_x + 1,
            y: max_y + 1,
        };

        (tiles, map_size, existing_tilemaps)
    };

    let world = app.world_mut();
    for entity in existing_tilemaps {
        world.entity_mut(entity).despawn();
    }

    let mut tile_storage = TileStorage::empty(map_size);
    for (entity, pos) in tiles {
        tile_storage.set(&pos, entity);
    }

    world.spawn((tile_storage, map_size)).id()
}

/// Loads a test fixture into the app. Returns true if load completed.
pub fn load_fixture(app: &mut bevy::app::App, fixture_name: &str) -> bool {
    let path = fixture_path(fixture_name);

    if !path.exists() {
        panic!(
            "Fixture not found: {:?}. Run 'cargo run --bin generate_test_map' to generate it.",
            path
        );
    }

    // Reset load completion flag
    app.world_mut().resource_mut::<FixtureLoadCompleted>().0 = false;

    // Trigger load
    app.world_mut()
        .commands()
        .trigger_load(LoadWorld::default_from_file(path));

    // Run updates until load completes or a timeout is reached
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        app.update();

        // Check if load completed via our observer
        if app.world().resource::<FixtureLoadCompleted>().0 {
            return true;
        }
    }

    false
}
