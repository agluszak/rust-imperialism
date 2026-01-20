use bevy::prelude::*;

use crate::ai::AiPlugin;
use crate::civilians::{CivilianPlugin, CivilianRenderingPlugin};
use crate::diplomacy::DiplomacyPlugin;
use crate::economy::EconomyPlugin;
use crate::helpers::camera::CameraPlugin;
use crate::helpers::picking::TilemapBackend;
use crate::input::InputPlugin;
use crate::map::MapLogicPlugin;
use crate::map::rendering::{
    BorderRenderingPlugin, CityRenderingPlugin, ImprovementRenderingPlugin, MapRenderingPlugin,
    ProspectingMarkersPlugin, TransportDebugPlugin, TransportRenderingPlugin,
};
use crate::save::GameSavePlugin;
use crate::ships::ShipsPlugin;
use crate::turn_system::TurnSystemPlugin;
use crate::ui::GameUIPlugin;

/// Core game logic plugins (no rendering or player input).
pub struct LogicPlugins;

impl Plugin for LogicPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapLogicPlugin,
            TurnSystemPlugin,
            EconomyPlugin,
            ShipsPlugin,
            AiPlugin,
            CivilianPlugin,
            DiplomacyPlugin,
            GameSavePlugin,
        ));
    }
}

/// Map rendering plugins (tilemap + visual layers).
pub struct MapRenderingPlugins;

impl Plugin for MapRenderingPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapRenderingPlugin,
            TransportRenderingPlugin,
            TransportDebugPlugin,
            BorderRenderingPlugin,
            CityRenderingPlugin,
            ImprovementRenderingPlugin,
            ProspectingMarkersPlugin,
            CivilianRenderingPlugin,
        ));
    }
}

/// Player input and UI plugins.
pub struct PlayerInputPlugins;

impl Plugin for PlayerInputPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((TilemapBackend, CameraPlugin, GameUIPlugin, InputPlugin));
    }
}
