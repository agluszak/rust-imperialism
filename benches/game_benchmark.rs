use bevy::ecs::message::MessageWriter;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::TilemapTexture;
use criterion::{Criterion, criterion_group, criterion_main};
use rust_imperialism::LogicPlugins;
use rust_imperialism::app;
use rust_imperialism::map::MapGenerationPlugin;
use rust_imperialism::turn_system::{EndPlayerTurn, TurnCounter};
use rust_imperialism::ui::components::MapTilemap;
use rust_imperialism::ui::menu::AppState;

fn setup_headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins((LogicPlugins, MapGenerationPlugin));

    // Transition to InGame state directly
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);

    // Run once to process state transition
    app.update();

    // Run until map is created
    let mut i = 0;
    loop {
        app.update();
        let map_ready = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<MapTilemap>>()
                .iter(world)
                .next()
                .is_some()
        };
        if map_ready {
            break;
        }
        i += 1;
        if i > 1000 {
            panic!("Map generation timed out");
        }
    }

    app
}

fn bench_headless_turns(c: &mut Criterion) {
    let mut app = setup_headless_app();

    c.bench_function("headless_turn", |b| {
        b.iter(|| {
            // Get current turn
            let start_turn = app.world().resource::<TurnCounter>().current;

            // End player turn
            let _ = app
                .world_mut()
                .run_system_once(|mut writer: MessageWriter<EndPlayerTurn>| {
                    writer.write(EndPlayerTurn);
                });

            // Process until turn increases
            // We limit the loop to avoid infinite loops in case of error
            let mut limit = 0;
            loop {
                app.update();
                let current_turn = app.world().resource::<TurnCounter>().current;
                if current_turn > start_turn {
                    break;
                }
                limit += 1;
                if limit > 10000 {
                    panic!("Turn advancement timed out");
                }
            }
        })
    });
}

fn bench_graphics_rendering(c: &mut Criterion) {
    // This benchmark requires a display (DefaultPlugins).
    // If running in a headless CI environment without Xvfb/Wayland, this might fail.
    // We check for display environment variables to avoid panics in CI.
    if std::env::var("DISPLAY").is_err()
        && std::env::var("WAYLAND_DISPLAY").is_err()
        && std::env::var("WAYLAND_SOCKET").is_err()
    {
        return;
    }

    let mut app = app();

    // Transition to game
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();

    // Wait for map AND rendering initialization
    // MapTilemap means logic setup is done.
    // TilemapTexture means rendering setup is done.
    let mut i = 0;
    loop {
        app.update();
        let map_ready = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<MapTilemap>>()
                .iter(world)
                .next()
                .is_some()
        };
        let rendering_ready = {
            let world = app.world_mut();
            world
                .query::<&TilemapTexture>()
                .iter(world)
                .next()
                .is_some()
        };

        if map_ready && rendering_ready {
            break;
        }
        i += 1;
        if i > 5000 {
            panic!("Map generation or rendering setup timed out");
        }
    }

    c.bench_function("graphics_rendering", |b| {
        b.iter(|| {
            app.update();
        })
    });
}

criterion_group!(benches, bench_headless_turns, bench_graphics_rendering);
criterion_main!(benches);
