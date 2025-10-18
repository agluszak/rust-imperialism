pub mod button_style;
pub mod city;
pub mod components;
pub mod diplomacy;
pub mod generic_systems;
pub mod input;
pub mod logging;
pub mod market;
pub mod menu;
pub mod mode;
pub mod setup;
pub mod state;
pub mod status;
pub mod transport;

use crate::ui::menu::AppState;
use bevy::prelude::*;

pub use components::ScrollableTerminal;
pub use input::handle_mouse_wheel_scroll;
// Do not expose the logging resource outside the module; consumers should send events instead.
// pub use logging::TerminalLog;

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
        .insert_resource(logging::TerminalLog::new(100))
        .insert_resource(state::UIState::default())
        .add_message::<logging::TerminalLogEvent>()
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
        // Initialize terminal log messages once at startup
        .add_systems(Startup, logging::setup_terminal_log)
        .add_systems(
            Update,
            (
                // State management runs first to collect current game state
                state::collect_ui_state,
                state::notify_ui_state_changes.after(state::collect_ui_state),
                // UI update systems run after state collection
                // Consume log events before updating UI text so new lines appear
                logging::consume_log_events.after(state::notify_ui_state_changes),
                status::update_turn_display.after(state::notify_ui_state_changes),
                status::update_calendar_display,
                status::update_treasury_display,
                status::update_tile_info_display,
                logging::update_terminal_output.after(logging::consume_log_events),
                // Mouse wheel scroll input handling
                input::handle_mouse_wheel_scroll,
                // Clamp scroll position after all scroll operations
                input::clamp_scroll_position.after(input::handle_mouse_wheel_scroll),
                // Mode buttons handler (only active in-game)
                mode::handle_mode_buttons.run_if(in_state(AppState::InGame)),
                // Button interaction visual feedback
                button_style::button_interaction_system,
                button_style::accent_button_interaction_system,
                button_style::danger_button_interaction_system,
            ),
        );
    }
}
