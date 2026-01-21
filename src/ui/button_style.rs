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

/// System that handles button interaction visual feedback
/// Updates BackgroundColor based on Interaction state
pub fn button_interaction_system(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

/// Marker component for accent-styled buttons
#[derive(Component)]
pub struct AccentButton;

/// System that handles accent button interaction visual feedback
pub fn accent_button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<AccentButton>),
    >,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_ACCENT.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_ACCENT.into();
            }
            Interaction::None => {
                *color = NORMAL_ACCENT.into();
            }
        }
    }
}

/// Marker component for danger-styled buttons
#[derive(Component)]
pub struct DangerButton;

/// System that handles danger button interaction visual feedback
pub fn danger_button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<DangerButton>),
    >,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_DANGER.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_DANGER.into();
            }
            Interaction::None => {
                *color = NORMAL_DANGER.into();
            }
        }
    }
}
