use bevy::ecs::message::MessageWriter;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use criterion::{criterion_group, criterion_main, Criterion};
use rust_imperialism::app;
use rust_imperialism::map::{MapGenerationPlugin, TilemapCreated, TilemapRenderingInitialized};
use rust_imperialism::turn_system::{EndPlayerTurn, TurnCounter};
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::LogicPlugins;

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
        if app.world().contains_resource::<TilemapCreated>() {
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
            let _ = app.world_mut()
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
    // TilemapCreated means logic is done.
    // TilemapRenderingInitialized means meshes/assets are ready.
    let mut i = 0;
    loop {
        app.update();
        let map_ready = app.world().contains_resource::<TilemapCreated>();
        let rendering_ready = app
            .world_mut()
            .query::<&TilemapRenderingInitialized>()
            .iter(app.world())
            .next()
            .is_some();

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
