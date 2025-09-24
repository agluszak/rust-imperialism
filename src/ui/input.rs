use bevy::prelude::*;

use crate::ui::components::ScrollableTerminal;
use crate::ui::components::TerminalOutput;
use crate::ui::metrics::ScrollbarMetrics;

pub fn handle_mouse_wheel_scroll(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut scrollable_query: Query<
        (
            &mut ScrollPosition,
            &bevy::ui::RelativeCursorPosition,
            &ComputedNode,
        ),
        With<ScrollableTerminal>,
    >,
    terminal_text_query: Query<(&TextFont, &ComputedNode), With<TerminalOutput>>,
) {
    for event in scroll_events.read() {
        for (mut scroll_position, cursor_position, computed) in scrollable_query.iter_mut() {
            // Only scroll if mouse is over the terminal
            if let Some(pos) = cursor_position.normalized
                && pos.x >= 0.0 && pos.x <= 1.0 && pos.y >= 0.0 && pos.y <= 1.0 {
                    let visible_size = computed.size();
                    let (font_size, actual_content_size) =
                        if let Ok((text_font, text_computed)) = terminal_text_query.single() {
                            (text_font.font_size, Some(text_computed.content_size()))
                        } else {
                            (12.0, None)
                        };

                    let metrics = ScrollbarMetrics::calculate_with_content_size(
                        visible_size,
                        font_size,
                        actual_content_size,
                    );

                    if metrics.can_scroll {
                        let scroll_amount = event.y * (font_size * 2.0); // Scroll by 2 lines at a time
                        let new_scroll_y = scroll_position.offset_y - scroll_amount;
                        scroll_position.offset_y = metrics.clamp_scroll_position(new_scroll_y);
                    }
                    return; // Terminal scrolled, don't process more events
                }
        }
    }
}
