pub mod components;
pub mod input;
pub mod logging;
pub mod metrics;
pub mod scrollbar;
pub mod setup;
pub mod state;
pub mod status;

use bevy::prelude::*;

pub use components::{ScrollableTerminal, ScrollbarThumb, ScrollbarTrack};
pub use input::handle_mouse_wheel_scroll;
// Do not expose the logging resource outside the module; consumers should send events instead.
// pub use logging::TerminalLog;

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(logging::TerminalLog::new(100))
            .insert_resource(state::UIState::default())
            .add_event::<logging::TerminalLogEvent>()
            .add_event::<state::UIStateUpdated>()
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
                    // Scrollbar systems run independently
                    scrollbar::handle_scrollbar_drag,
                    input::handle_mouse_wheel_scroll,
                    scrollbar::update_scrollbar
                        .after(scrollbar::handle_scrollbar_drag)
                        .after(input::handle_mouse_wheel_scroll),
                    scrollbar::update_scrollbar_during_drag.after(scrollbar::handle_scrollbar_drag),
                    scrollbar::clamp_scroll_position,
                ),
            );
    }
}
