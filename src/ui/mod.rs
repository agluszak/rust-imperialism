pub mod button_style;
pub mod city;
pub mod components;
pub mod diplomacy;
pub mod generic_systems;
pub mod market;
pub mod menu;
pub mod mode;
pub mod setup;
pub mod state;
pub mod status;
pub mod transport;

use crate::ui::menu::AppState;
use bevy::prelude::*;

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        // Note: UiWidgetsPlugins (including ScrollbarPlugin, SliderPlugin, etc.)
        // is added in lib.rs, so we don't add individual widget plugins here
        app.add_plugins((
            city::CityUIPlugin,
            transport::TransportUIPlugin,
            market::MarketUIPlugin,
            diplomacy::DiplomacyUIPlugin,
            menu::MenuUIPlugin,
        ))
        .insert_resource(state::UIState::default())
        .add_message::<state::UIStateUpdated>()
        // Spawn gameplay UI only when entering InGame state
        .add_systems(OnEnter(AppState::InGame), setup::setup_ui)
        // Show/hide Map UI based on GameMode
        .add_systems(
            OnEnter(mode::GameMode::Map),
            (
                generic_systems::show_screen::<components::GameplayUIRoot>,
                generic_systems::show_screen::<components::MapTilemap>,
            ),
        )
        .add_systems(
            OnExit(mode::GameMode::Map),
            (
                generic_systems::hide_screen::<components::GameplayUIRoot>,
                generic_systems::hide_screen::<components::MapTilemap>,
            ),
        )
        .add_systems(
            Update,
            (
                // State management runs first to collect current game state
                state::collect_ui_state,
                state::notify_ui_state_changes.after(state::collect_ui_state),
                // UI update systems run after state collection
                status::update_turn_display.after(state::notify_ui_state_changes),
                status::update_calendar_display,
                status::update_treasury_display,
                status::update_tile_info_display,
                // Button interaction visual feedback (standard Button widget handles mode switching via observers)
                button_style::button_interaction_system,
                button_style::accent_button_interaction_system,
                button_style::danger_button_interaction_system,
            ),
        );
    }
}
