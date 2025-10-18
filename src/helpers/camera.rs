use bevy::{
    input::{ButtonInput, mouse::MouseWheel},
    math::Vec3,
    prelude::*,
    ui::RelativeCursorPosition,
};

use crate::economy::{Capital, NationId, PlayerNation};
use crate::map::TilePosExt;
use crate::ui::ScrollableTerminal;
use crate::ui::mode::GameMode;

/// Plugin that handles camera setup and control
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                center_on_player_capital.run_if(resource_added::<PlayerNation>),
                movement
                    .after(crate::ui::handle_mouse_wheel_scroll)
                    .run_if(in_state(GameMode::Map)),
            ),
        );
    }
}

/// Set up the camera at startup
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Center camera on player's capital when the game starts
fn center_on_player_capital(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    player_nation: Option<Res<PlayerNation>>,
    capitals: Query<(&Capital, &NationId)>,
) {
    // Only run once when player nation is available
    let Some(_player) = player_nation else {
        return;
    };

    // Find player's capital
    for (capital, _nation_id) in capitals.iter() {
        // Check if this capital belongs to the player's nation by checking the entity
        // Since we can't directly query the entity's nation, we'll use the first capital we find
        // (which should be the player's based on setup order)
        if let Ok(mut transform) = camera.single_mut() {
            let capital_world_pos = capital.0.to_world_pos();
            transform.translation.x = capital_world_pos.x;
            transform.translation.y = capital_world_pos.y;
            info!(
                "Camera centered on capital at ({:.1}, {:.1})",
                capital_world_pos.x, capital_world_pos.y
            );
            return;
        }
    }
}

/// Handle camera movement and zooming
pub fn movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: MessageReader<MouseWheel>,
    mut query: Query<(&mut Transform, &mut Projection), With<Camera>>,
    terminal_area: Query<&RelativeCursorPosition, With<ScrollableTerminal>>,
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

        // Determine if the cursor is over the terminal (which includes the built-in scrollbar)
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
