use bevy::{
    input::{ButtonInput, mouse::MouseWheel},
    math::Vec3,
    prelude::*,
    render::camera::Camera,
    ui::RelativeCursorPosition,
};

use crate::ui::{ScrollableTerminal, ScrollbarThumb, ScrollbarTrack};

// A simple camera system for moving and zooming the camera.
#[allow(dead_code)]
pub fn movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut query: Query<(&mut Transform, &mut Projection), With<Camera>>,
    terminal_area: Query<&RelativeCursorPosition, With<ScrollableTerminal>>,
    scrollbar_track: Query<&RelativeCursorPosition, With<ScrollbarTrack>>,
    scrollbar_thumb: Query<&RelativeCursorPosition, With<ScrollbarThumb>>,
) {
    for (mut transform, mut projection) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyA) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::KeyS) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }

        let Projection::Orthographic(ortho) = &mut *projection else {
            continue;
        };

        // Determine if the cursor is over any UI that should capture scrolling (terminal or its scrollbar)
        let mut cursor_over_ui = false;
        for cursor in terminal_area.iter() {
            if let Some(pos) = cursor.normalized
                && pos.x >= 0.0
                && pos.x <= 1.0
                && pos.y >= 0.0
                && pos.y <= 1.0
            {
                cursor_over_ui = true;
                break;
            }
        }
        if !cursor_over_ui {
            for cursor in scrollbar_track.iter() {
                if let Some(pos) = cursor.normalized
                    && pos.x >= 0.0
                    && pos.x <= 1.0
                    && pos.y >= 0.0
                    && pos.y <= 1.0
                {
                    cursor_over_ui = true;
                    break;
                }
            }
        }
        if !cursor_over_ui {
            for cursor in scrollbar_thumb.iter() {
                if let Some(pos) = cursor.normalized
                    && pos.x >= 0.0
                    && pos.x <= 1.0
                    && pos.y >= 0.0
                    && pos.y <= 1.0
                {
                    cursor_over_ui = true;
                    break;
                }
            }
        }

        // Handle mouse wheel zooming, but ignore when cursor is over terminal UI
        for ev in scroll_evr.read() {
            if cursor_over_ui {
                continue;
            }
            let zoom_factor = if ev.y > 0.0 { 0.9 } else { 1.1 };
            ortho.scale *= zoom_factor;
        }

        // Handle keyboard zooming (Z to zoom out, X to zoom in)
        if keyboard_input.pressed(KeyCode::KeyZ) {
            ortho.scale += 0.1 * time.delta_secs() * 5.0; // Smooth zooming
        }

        if keyboard_input.pressed(KeyCode::KeyX) {
            ortho.scale -= 0.1 * time.delta_secs() * 5.0; // Smooth zooming
        }

        // Clamp zoom levels to reasonable bounds
        ortho.scale = ortho.scale.clamp(0.1, 5.0);

        // Scale movement speed based on zoom level for consistent feel
        let movement_speed = 500.0 * ortho.scale;

        let z = transform.translation.z;
        transform.translation += time.delta_secs() * direction * movement_speed;
        // Important! We need to restore the Z values when moving the camera around.
        // Bevy has a specific camera setup and this can mess with how our layers are shown.
        transform.translation.z = z;
    }
}
