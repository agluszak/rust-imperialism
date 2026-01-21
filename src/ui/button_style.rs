use bevy::prelude::*;

/// Standard button color constants following Bevy UI conventions
pub const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

/// Accent button colors (for primary actions)
pub const NORMAL_ACCENT: Color = Color::srgb(0.2, 0.3, 0.25);
pub const HOVERED_ACCENT: Color = Color::srgb(0.3, 0.45, 0.35);
pub const PRESSED_ACCENT: Color = Color::srgb(0.35, 0.75, 0.35);

/// Danger button colors (for destructive actions)
pub const NORMAL_DANGER: Color = Color::srgb(0.3, 0.15, 0.15);
pub const HOVERED_DANGER: Color = Color::srgb(0.45, 0.2, 0.2);
pub const PRESSED_DANGER: Color = Color::srgb(0.75, 0.35, 0.35);

/// Button style helper for creating consistent button nodes
pub fn button_node() -> Node {
    Node {
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

/// Marker component for accent-styled buttons
#[derive(Component)]
pub struct AccentButton;

/// Marker component for danger-styled buttons
#[derive(Component)]
pub struct DangerButton;

/// System that handles button interaction visual feedback for all button types
/// Updates BackgroundColor based on Interaction state and button type markers
pub fn unified_button_interaction_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&AccentButton>,
            Option<&DangerButton>,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut color, accent, danger) in interaction_query.iter_mut() {
        let (normal, hovered, pressed) = if accent.is_some() {
            (NORMAL_ACCENT, HOVERED_ACCENT, PRESSED_ACCENT)
        } else if danger.is_some() {
            (NORMAL_DANGER, HOVERED_DANGER, PRESSED_DANGER)
        } else {
            (NORMAL_BUTTON, HOVERED_BUTTON, PRESSED_BUTTON)
        };

        match *interaction {
            Interaction::Pressed => {
                *color = pressed.into();
            }
            Interaction::Hovered => {
                *color = hovered.into();
            }
            Interaction::None => {
                *color = normal.into();
            }
        }
    }
}
