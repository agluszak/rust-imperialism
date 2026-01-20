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
use crate::map::{MapRenderingPlugin, ProvinceGenerationPlugin};
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
use bevy::app::PluginGroup;

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

/// Plugin group for core game logic (headless-compatible)
/// Use this for tests that don't need rendering or player input
pub struct LogicPlugins;

impl PluginGroup for LogicPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(TurnSystemPlugin)
            .add(EconomyPlugin)
            .add(ShipsPlugin)
            .add(AiPlugin)
            .add(CivilianPlugin)
            .add(DiplomacyPlugin)
            .add(GameSavePlugin)
    }
}

/// Plugin group for map rendering (requires graphics/window)
/// Use this with LogicPlugins for visual output without player interaction
pub struct MapRenderingPlugins;

impl PluginGroup for MapRenderingPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(MapRenderingPlugin)
            .add(CameraPlugin)
            .add(TransportRenderingPlugin)
            .add(TransportDebugPlugin)
            .add(BorderRenderingPlugin)
            .add(CityRenderingPlugin)
            .add(ImprovementRenderingPlugin)
            .add(ProspectingMarkersPlugin)
            .add(CivilianRenderingPlugin)
    }
}

/// Plugin group for player input and UI (requires user interaction)
/// Use this with LogicPlugins and MapRenderingPlugins for full game
pub struct InputPlugins;

impl PluginGroup for InputPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(GameUIPlugin)
            .add(InputPlugin)
    }
}

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
        // Game plugins - organized into three groups
        .add_plugins(TilemapBackend)
        .add_plugins(LogicPlugins)
        .add_plugins(ProvinceGenerationPlugin) // Needed for full game map generation
        .add_plugins(MapRenderingPlugins)
        .add_plugins(InputPlugins);

    #[cfg(feature = "debug")]
    app.add_plugins((EguiPlugin::default(), WorldInspectorPlugin::new()));

    app
}

#[cfg(test)]
pub mod test_utils;
