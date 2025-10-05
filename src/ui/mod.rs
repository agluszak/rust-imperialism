pub mod components;
pub mod input;
pub mod logging;
pub mod setup;
pub mod state;
pub mod status;

use bevy::prelude::*;
use bevy::ui_widgets::ScrollbarPlugin;

pub use components::ScrollableTerminal;
pub use input::{clamp_scroll_position, handle_mouse_wheel_scroll};
// Do not expose the logging resource outside the module; consumers should send events instead.
// pub use logging::TerminalLog;

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScrollbarPlugin)
            .insert_resource(logging::TerminalLog::new(100))
            .insert_resource(state::UIState::default())
            .add_message::<logging::TerminalLogEvent>()
            .add_message::<state::UIStateUpdated>()
            .add_systems(Startup, (setup::setup_ui, logging::setup_terminal_log))
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
                    status::update_hero_status_display.after(state::notify_ui_state_changes),
                    status::update_monster_count_display.after(state::notify_ui_state_changes),
                    logging::update_terminal_output.after(logging::consume_log_events),
                    // Mouse wheel scroll input handling
                    input::handle_mouse_wheel_scroll,
                    // Clamp scroll position after all scroll operations
                    input::clamp_scroll_position.after(input::handle_mouse_wheel_scroll),
                ),
            );
    }
}
