//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

use crate::plugins::{LogicPlugins, MapRenderingPlugins, PlayerInputPlugins};
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;
#[cfg(feature = "debug")]
use bevy::dev_tools::states::log_transitions;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;

#[cfg(feature = "debug")]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(feature = "debug")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub mod ai;
pub mod assets;
pub mod bmp_loader;
pub mod civilians;
pub mod constants;
pub mod debug;
pub mod diplomacy;
pub mod economy;
pub mod helpers;
pub mod input;
pub mod map;
pub mod messages;
pub mod orders;
pub mod plugins;
pub mod resources;
pub mod save;
pub mod ships;
pub mod turn_system;
pub mod ui;

pub fn app() -> App {
    let mut app = App::new();

    app
        // Core Bevy plugins
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            bmp_loader::ImperialismBmpLoaderPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::UiWidgetsPlugins,
            TilemapPlugin,
        ))
        // App state management
        .insert_state(AppState::MainMenu)
        .add_sub_state::<GameMode>();

    #[cfg(feature = "debug")]
    app.add_systems(Update, log_transitions::<AppState>)
        .add_systems(Update, log_transitions::<GameMode>);

    app
        // Game plugins
        .add_plugins((LogicPlugins, MapRenderingPlugins, PlayerInputPlugins));

    #[cfg(feature = "debug")]
    app.add_plugins((EguiPlugin::default(), WorldInspectorPlugin::new()));

    app
}

#[cfg(test)]
pub mod test_utils;
