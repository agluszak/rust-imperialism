use bevy::prelude::*;

use crate::ui::mode::GameMode;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::City), ensure_city_screen_visible)
            .add_systems(OnExit(GameMode::City), hide_city_screen);
    }
}

pub fn ensure_city_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<CityScreen>>,
) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    // Fullscreen city background panel
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
            BackgroundColor(Color::srgba(0.07, 0.07, 0.1, 0.95)),
            CityScreen,
            Visibility::Visible,
        ))
        .with_children(|parent| {
            // Header
            parent.spawn((
                Text::new("City Overview"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.8)),
            ));

            // Return to Map button (top-right)
            parent
                .spawn((
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
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Back to Map"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });

            // Buildings panel (inspired by the reference image)
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.15, 0.9)),
                ))
                .with_children(|buildings| {
                    // Textile Mill row
                    buildings
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                row_gap: Val::Px(6.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.15, 0.18, 0.9)),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Textile Mill  —  Capacity: 8"),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.95, 1.0)),
                            ));
                            row.spawn((
                                Text::new("Inputs: 1x Wool, 1x Cotton   →   Produces: Cloth"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                            row.spawn((
                                Node {
                                    width: Val::Px(260.0),
                                    height: Val::Px(14.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
                                BorderColor::all(Color::srgba(0.6, 0.6, 0.7, 1.0)),
                            ))
                            .with_children(|bar| {
                                bar.spawn((
                                    Node {
                                        width: Val::Percent(75.0), // 6/8 ≈ 75%
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.3, 0.7, 0.3, 1.0)),
                                ));
                            });
                        });

                    // Clothing Factory row
                    buildings
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                row_gap: Val::Px(6.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.15, 0.18, 0.9)),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Clothing Factory  —  Capacity: 16"),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.95, 1.0)),
                            ));
                            row.spawn((
                                Text::new("Inputs: 1x Cloth   →   Produces: Clothes"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                            row.spawn((
                                Node {
                                    width: Val::Px(260.0),
                                    height: Val::Px(14.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
                                BorderColor::all(Color::srgba(0.6, 0.6, 0.7, 1.0)),
                            ))
                            .with_children(|bar| {
                                bar.spawn((
                                    Node {
                                        width: Val::Percent(6.25), // 1/16 ≈ 6.25%
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.3, 0.7, 0.3, 1.0)),
                                ));
                            });
                        });
                });
        });
}

pub fn hide_city_screen(mut roots: Query<&mut Visibility, With<CityScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
