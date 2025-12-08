//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

use bevy::dev_tools::states::log_transitions;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;

use crate::ai::{AiBehaviorPlugin, AiEconomyPlugin, AiSupportPlugin};
use crate::civilians::CivilianPlugin;
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
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

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
pub mod turn_system;
pub mod ui;

pub fn app() -> App {
    let mut app = App::new();

    app
        // Core Bevy plugins
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            bmp_loader::ImperialismBmpLoaderPlugin,
            bevy::ui_widgets::UiWidgetsPlugins,
            TilemapPlugin,
        ))
        // App state management
        .insert_state(AppState::MainMenu)
        .add_sub_state::<GameMode>()
        .add_systems(Update, log_transitions::<AppState>)
        .add_systems(Update, log_transitions::<GameMode>)
        // Game plugins
        .add_plugins((
            TilemapBackend,
            CameraPlugin,
            MapSetupPlugin,
            TurnSystemPlugin,
            EconomyPlugin,
            AiSupportPlugin,
            AiEconomyPlugin,
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
        ))
        .add_plugins(GameSavePlugin)
        .add_plugins(AiBehaviorPlugin);

    app
}

#[cfg(test)]
pub mod test_utils;
