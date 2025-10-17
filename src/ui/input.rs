use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::ui::components::{ScrollableTerminal, TerminalOutput};

/// Clamps scroll position to prevent overscrolling
pub fn clamp_scroll_position(
    mut scrollable_query: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollableTerminal>>,
    terminal_text_query: Query<&ComputedNode, With<TerminalOutput>>,
) {
    for (mut scroll_position, computed) in scrollable_query.iter_mut() {
        let visible_height = computed.size().y;
        let content_height = terminal_text_query
            .iter()
            .next()
            .map(|node| node.content_size().y)
            .unwrap_or(visible_height);

        let max_scroll = (content_height - visible_height).max(0.0);
        scroll_position.y = scroll_position.y.clamp(0.0, max_scroll);
    }
}

pub fn handle_mouse_wheel_scroll(
    mut scroll_events: MessageReader<MouseWheel>,
    mut scrollable_query: Query<
        (&mut ScrollPosition, &RelativeCursorPosition, &ComputedNode),
        With<ScrollableTerminal>,
    >,
    terminal_text_query: Query<&ComputedNode, With<TerminalOutput>>,
) {
    for event in scroll_events.read() {
        for (mut scroll_position, cursor_position, computed) in scrollable_query.iter_mut() {
            // Only scroll if mouse is over the terminal
            if let Some(pos) = cursor_position.normalized
                && pos.x >= 0.0
                && pos.x <= 1.0
                && pos.y >= 0.0
                && pos.y <= 1.0
            {
                // Scroll by approximately 2 lines at a time (assuming 12px font size)
                let scroll_amount = event.y * 24.0;
                let new_scroll_y = scroll_position.y - scroll_amount;

                // Clamp scroll position to valid bounds
                let visible_height = computed.size().y;
                let content_height = terminal_text_query
                    .iter()
                    .next()
                    .map(|node| node.content_size().y)
                    .unwrap_or(visible_height);

                let max_scroll = (content_height - visible_height).max(0.0);
                scroll_position.y = new_scroll_y.clamp(0.0, max_scroll);

                return; // Terminal scrolled, don't process more events
            }
        }
    }
}
