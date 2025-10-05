use bevy::prelude::*;

use crate::ui::mode::GameMode;

#[derive(Component)]
pub struct DiplomacyScreen;

pub struct DiplomacyUIPlugin;

impl Plugin for DiplomacyUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::Diplomacy), ensure_diplomacy_screen_visible)
            .add_systems(OnExit(GameMode::Diplomacy), hide_diplomacy_screen);
    }
}

pub fn ensure_diplomacy_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<DiplomacyScreen>>,
) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.05, 0.05, 0.92)),
            DiplomacyScreen,
            Visibility::Visible,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Diplomacy Mode (stub)"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.95, 0.9, 1.0)),
            ));

            // Back to Map
            parent.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                crate::ui::mode::MapModeButton,
            )).with_children(|b| {
                b.spawn((
                    Text::new("Back to Map"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                ));
            });
        });
}

pub fn hide_diplomacy_screen(mut roots: Query<&mut Visibility, With<DiplomacyScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
