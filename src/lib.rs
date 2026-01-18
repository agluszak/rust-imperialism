//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

use crate::ai::AiPlugin;
use crate::civilians::{CivilianPlugin, CivilianRenderingPlugin};
use crate::diplomacy::DiplomacyPlugin;
use crate::economy::EconomyPlugin;
use crate::helpers::camera::CameraPlugin;
use crate::helpers::picking::TilemapBackend;
use crate::input::InputPlugin;
use crate::map::MapSetupPlugin;
use crate::map::rendering::border_rendering::BorderRenderingPlugin;
use crate::map::rendering::city_rendering::CityRenderingPlugin;
use crate::map::rendering::improvement_rendering::ImprovementRenderingPlugin;
use crate::map::rendering::prospecting_markers::ProspectingMarkersPlugin;
use crate::map::rendering::{TransportDebugPlugin, TransportRenderingPlugin};
use crate::save::GameSavePlugin;
use crate::ships::ShipsPlugin;
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;
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
        .add_plugins((
            TilemapBackend,
            CameraPlugin,
            MapSetupPlugin,
            TurnSystemPlugin,
            EconomyPlugin,
            ShipsPlugin,
            AiPlugin, // New unified AI plugin
            CivilianPlugin,
            DiplomacyPlugin,
        ))
        .add_plugins((
            GameUIPlugin,
            InputPlugin,
            TransportRenderingPlugin,
            TransportDebugPlugin,
            BorderRenderingPlugin,
            CityRenderingPlugin,
            ImprovementRenderingPlugin,
            ProspectingMarkersPlugin,
            CivilianRenderingPlugin,
        ))
        .add_plugins(GameSavePlugin);

    #[cfg(feature = "debug")]
    app.add_plugins((EguiPlugin::default(), WorldInspectorPlugin::new()));

    app
}

#[cfg(test)]
pub mod test_utils;
