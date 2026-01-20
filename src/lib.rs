//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

pub use crate::ai::AiPlugin;
pub use crate::civilians::CivilianLogicPlugin;
pub use crate::diplomacy::DiplomacyPlugin;
pub use crate::economy::EconomyPlugin;
pub use crate::helpers::camera::CameraPlugin;
pub use crate::helpers::picking::TilemapBackend;
pub use crate::input::InputPlugin;
pub use crate::map::{MapGenerationPlugin, MapLogicPlugin};
pub use crate::map::rendering::MapRenderingPlugin;
use crate::save::GameSavePlugin;
use crate::ships::ShipsPlugin;
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;
use bevy::app::PluginGroupBuilder;
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

/// Plugin for core game state management
pub struct GameCorePlugin;

impl Plugin for GameCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<GameMode>();

        #[cfg(feature = "debug")]
        app.add_systems(Update, log_transitions::<AppState>)
            .add_systems(Update, log_transitions::<GameMode>);
    }
}

/// Group of plugins for core game logic
pub struct LogicPlugins;

impl PluginGroup for LogicPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(GameCorePlugin)
            .add(MapLogicPlugin)
            .add(TurnSystemPlugin)
            .add(EconomyPlugin)
            .add(ShipsPlugin)
            .add(AiPlugin)
            .add(CivilianLogicPlugin)
            .add(DiplomacyPlugin)
            .add(GameSavePlugin)
    }
}

/// Group of plugins for map and world rendering
pub struct MapRenderingPlugins;

impl PluginGroup for MapRenderingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(TilemapPlugin)
            .add(TilemapBackend)
            .add(CameraPlugin)
            .add(MapRenderingPlugin)
    }
}

/// Group of plugins for player input and UI
pub struct InputPlugins;

impl PluginGroup for InputPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BevyUiInputPlugin)
            .add(InputPlugin)
            .add(GameUIPlugin)
    }
}

/// Helper plugin to wrap Bevy's internal input/UI plugins which may be PluginGroups
struct BevyUiInputPlugin;

impl Plugin for BevyUiInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::UiWidgetsPlugins,
        ));
    }
}

pub fn app() -> App {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));

    app
        // Game plugins
        .add_plugins((
            LogicPlugins,
            MapGenerationPlugin,
            MapRenderingPlugins,
            InputPlugins,
        ));

    #[cfg(feature = "debug")]
    app.add_plugins((EguiPlugin::default(), WorldInspectorPlugin::new()));

    app
}

#[cfg(test)]
pub mod test_utils;
