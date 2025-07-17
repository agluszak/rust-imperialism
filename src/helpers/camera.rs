use bevy::{input::{ButtonInput, mouse::MouseWheel}, math::Vec3, prelude::*, render::camera::Camera};

// A simple camera system for moving and zooming the camera.
#[allow(dead_code)]
pub fn movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut query: Query<(&mut Transform, &mut Projection), With<Camera>>,
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

        // Handle mouse wheel zooming
        for ev in scroll_evr.read() {
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
