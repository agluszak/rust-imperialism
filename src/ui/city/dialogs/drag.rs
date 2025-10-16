use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::types::{DialogDragHandle, DialogDragState};

/// Start dragging when clicking on the dialog header (Input Layer)
pub fn start_dialog_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    drag_handles: Query<(&Interaction, &DialogDragHandle), Changed<Interaction>>,
    mut dialogs: Query<(&mut DialogDragState, &Node)>,
) {
    // Check if left mouse button was just pressed
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Find if we clicked on any drag handle
    for (interaction, handle) in drag_handles.iter() {
        if *interaction == Interaction::Pressed {
            // Start dragging this dialog
            if let Ok((mut drag_state, node)) = dialogs.get_mut(handle.dialog_entity) {
                drag_state.is_dragging = true;

                // Calculate offset from dialog top-left to cursor
                let dialog_left = match node.left {
                    Val::Px(px) => px,
                    _ => 0.0,
                };
                let dialog_top = match node.top {
                    Val::Px(px) => px,
                    _ => 0.0,
                };

                drag_state.drag_offset = Vec2::new(
                    cursor_position.x - dialog_left,
                    cursor_position.y - dialog_top,
                );
            }
        }
    }
}

/// Update dialog position while dragging (Logic Layer)
pub fn update_dialog_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut dialogs: Query<(&mut DialogDragState, &mut Node)>,
) {
    // Stop dragging if mouse button is released
    if !mouse_button.pressed(MouseButton::Left) {
        for (mut drag_state, _) in dialogs.iter_mut() {
            if drag_state.is_dragging {
                drag_state.is_dragging = false;
            }
        }
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Update position of any dialogs being dragged
    for (drag_state, mut node) in dialogs.iter_mut() {
        if drag_state.is_dragging {
            let new_left = cursor_position.x - drag_state.drag_offset.x;
            let new_top = cursor_position.y - drag_state.drag_offset.y;

            // Clamp to window bounds
            let window_width = window.width();
            let window_height = window.height();

            let dialog_width = match node.width {
                Val::Px(px) => px,
                _ => 380.0,
            };

            let clamped_left = new_left.max(0.0).min(window_width - dialog_width);
            let clamped_top = new_top.max(0.0).min(window_height - 100.0); // Keep at least 100px visible

            node.left = Val::Px(clamped_left);
            node.top = Val::Px(clamped_top);
        }
    }
}

/// Visual feedback for drag handle hover (Rendering Layer)
/// TODO: Update cursor icon when Bevy 0.17 cursor API is clarified
pub fn update_drag_handle_cursor(
    _drag_handles: Query<&Interaction, (Changed<Interaction>, With<DialogDragHandle>)>,
) {
    // Cursor icon changes disabled for now
    // Will be re-enabled once Bevy 0.17 cursor API is confirmed
}
