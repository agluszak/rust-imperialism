use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::ui::components::{
    ScrollableTerminal, ScrollbarDragStart, ScrollbarThumb, ScrollbarTrack, TerminalOutput,
};
use crate::ui::metrics::ScrollbarMetrics;

pub fn update_scrollbar(
    scrollable_query: Query<(&ScrollPosition, &Node, &ComputedNode), With<ScrollableTerminal>>,
    mut thumb_query: Query<
        &mut Node,
        (
            With<ScrollbarThumb>,
            Without<ScrollableTerminal>,
            Without<ScrollbarDragStart>,
        ),
    >,
    terminal_text_query: Query<(&TextFont, &ComputedNode), With<TerminalOutput>>,
) {
    for (scroll_position, _node, computed) in scrollable_query.iter() {
        let visible_size = computed.size();

        // Get font size and actual content size from terminal text component
        let (font_size, actual_content_size) =
            if let Ok((text_font, text_computed)) = terminal_text_query.single() {
                (text_font.font_size, Some(text_computed.content_size()))
            } else {
                (12.0, None) // fallback
            };

        let metrics = ScrollbarMetrics::calculate_with_content_size(
            visible_size,
            font_size,
            actual_content_size,
        );
        let clamped_scroll_y = metrics.clamp_scroll_position(scroll_position.offset_y);

        for mut thumb_node in thumb_query.iter_mut() {
            // Update thumb size based on content/viewport ratio
            thumb_node.height = Val::Percent((metrics.thumb_size_ratio * 100.0).max(5.0));

            // Update thumb position based on scroll position
            thumb_node.top = Val::Percent(metrics.thumb_position_percent(clamped_scroll_y));
        }
    }
}

pub fn update_scrollbar_during_drag(
    scrollable_query: Query<(&ScrollPosition, &Node, &ComputedNode), With<ScrollableTerminal>>,
    mut thumb_query: Query<
        &mut Node,
        (
            With<ScrollbarThumb>,
            Without<ScrollableTerminal>,
            With<ScrollbarDragStart>,
        ),
    >,
    terminal_text_query: Query<(&TextFont, &ComputedNode), With<TerminalOutput>>,
) {
    for (scroll_position, _node, computed) in scrollable_query.iter() {
        let visible_size = computed.size();

        // Get font size and actual content size from terminal text component
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
        let clamped_scroll_y = metrics.clamp_scroll_position(scroll_position.offset_y);

        for mut thumb_node in thumb_query.iter_mut() {
            // Update thumb position during drag - don't change size during drag
            thumb_node.top = Val::Percent(metrics.thumb_position_percent(clamped_scroll_y));
        }
    }
}

pub fn handle_scrollbar_drag(
    mut commands: Commands,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut scrollable_query: Query<
        (&mut ScrollPosition, &Node, &ComputedNode),
        With<ScrollableTerminal>,
    >,
    thumb_query: Query<(Entity, &RelativeCursorPosition), With<ScrollbarThumb>>,
    drag_query: Query<&ScrollbarDragStart, With<ScrollbarThumb>>,
    track_query: Query<&RelativeCursorPosition, (With<ScrollbarTrack>, Without<ScrollbarThumb>)>,
    terminal_text_query: Query<(&TextFont, &ComputedNode), With<TerminalOutput>>,
) {
    let mut thumb_clicked = false;

    // Check if thumb is being clicked first
    for (thumb_entity, cursor_position) in thumb_query.iter() {
        if mouse_button_input.just_pressed(MouseButton::Left)
            && let Some(pos) = cursor_position.normalized {
                // Only consider it a thumb click if cursor is actually over the thumb
                if pos.x >= 0.0 && pos.x <= 1.0 && pos.y >= 0.0 && pos.y <= 1.0 {
                    thumb_clicked = true;

                    if let Ok((scroll_position, _, _)) = scrollable_query.single() {
                        commands.entity(thumb_entity).insert(ScrollbarDragStart {
                            position: pos,
                            scroll_position: Vec2::new(
                                scroll_position.offset_x,
                                scroll_position.offset_y,
                            ),
                        });
                    }
                }
            }
    }

    // Handle clicking on scrollbar track (jump to position) only if thumb wasn't clicked
    if !thumb_clicked && mouse_button_input.just_pressed(MouseButton::Left) {
        for track_cursor in track_query.iter() {
            if let Some(pos) = track_cursor.normalized {
                // Only handle track clicks if cursor is actually over the track
                if pos.x >= 0.0 && pos.x <= 1.0 && pos.y >= 0.0 && pos.y <= 1.0
                    && let Ok((mut scroll_position, _node, computed)) =
                        scrollable_query.single_mut()
                    {
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
                            let new_scroll_y = pos.y * metrics.max_scroll;
                            scroll_position.offset_y = metrics.clamp_scroll_position(new_scroll_y);
                        }
                    }
            }
        }
    }

    // Handle dragging
    for (thumb_entity, _cursor_position) in thumb_query.iter() {
        if mouse_button_input.pressed(MouseButton::Left)
            && let Ok(drag_start) = drag_query.get(thumb_entity) {
                // Use the track's cursor position for dragging so we can move across the full range
                if let Ok(track_cursor) = track_query.get_single()
                    && let Some(track_pos) = track_cursor.normalized
                        && let Ok((mut scroll_position, _node, computed)) =
                            scrollable_query.single_mut()
                        {
                            let visible_size = computed.size();
                            let (font_size, actual_content_size) = if let Ok((
                                text_font,
                                text_computed,
                            )) =
                                terminal_text_query.single()
                            {
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
                                let thumb_height_percent = metrics.thumb_size_ratio * 100.0;
                                let max_thumb_travel = 100.0 - thumb_height_percent;

                                // Keep the initial grab offset inside the thumb so it feels natural
                                let mut desired_top_percent = (track_pos.y * 100.0)
                                    - (drag_start.position.y * thumb_height_percent);
                                if max_thumb_travel > 0.0 {
                                    desired_top_percent =
                                        desired_top_percent.clamp(0.0, max_thumb_travel);
                                    let scroll_ratio = desired_top_percent / max_thumb_travel;
                                    let new_scroll_y = scroll_ratio * metrics.max_scroll;
                                    scroll_position.offset_y =
                                        metrics.clamp_scroll_position(new_scroll_y);
                                } else {
                                    scroll_position.offset_y = 0.0;
                                }
                            }
                        }
            }

        // End dragging
        if mouse_button_input.just_released(MouseButton::Left) {
            Commands::entity(&mut commands, thumb_entity).remove::<ScrollbarDragStart>();
        }
    }
}

pub fn clamp_scroll_position(
    mut scrollable_query: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollableTerminal>>,
    terminal_text_query: Query<(&TextFont, &ComputedNode), With<TerminalOutput>>,
) {
    for (mut scroll_position, computed) in scrollable_query.iter_mut() {
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

        // Always clamp the scroll position to valid bounds
        scroll_position.offset_y = metrics.clamp_scroll_position(scroll_position.offset_y);
    }
}
