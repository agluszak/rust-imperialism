use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::ui_widgets::{ControlOrientation, CoreScrollbarThumb, Scrollbar};

use crate::ui::button_style::*;
use crate::ui::components::{
    CalendarDisplay, GameplayUIRoot, MapTilemap, ScrollableTerminal, TerminalOutput,
    TerminalWindow, TileInfoDisplay, TreasuryDisplay, TurnDisplay,
};

pub fn setup_ui(mut commands: Commands) {
    // Create HUD panel
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(10.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
        BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.8)),
        GameplayUIRoot,
        children![
            // Turn info section
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                children![
                    (
                        Text::new("Turn: 1 - Player Turn"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                        TurnDisplay,
                    ),
                    (
                        Text::new("Spring, 1815"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        CalendarDisplay,
                    ),
                    (
                        Text::new("$50,000"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TreasuryDisplay,
                    )
                ],
            ),
        ],
    ));

    // Create terminal window
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(420.0),
                height: Val::Px(300.0),
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(5.0)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            TerminalWindow,
            GameplayUIRoot,
        ))
        .with_children(|parent| {
            // Terminal header
            parent.spawn((
                Text::new("Terminal Output"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));

            // Container for scrollable content and scrollbar
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|container| {
                    // Scrollable content area
                    let scrollable_id = container
                        .spawn((
                            Node {
                                width: Val::Percent(95.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition::default(),
                            ScrollableTerminal,
                            RelativeCursorPosition::default(),
                            children![(
                                Text::new(""),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                                TerminalOutput,
                                Node {
                                    align_self: AlignSelf::Stretch,
                                    ..default()
                                },
                            )],
                        ))
                        .id();

                    // Headless scrollbar
                    container.spawn((
                        Node {
                            width: Val::Percent(5.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                        Scrollbar::new(scrollable_id, ControlOrientation::Vertical, 20.0),
                        children![(
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(20.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                            CoreScrollbarThumb,
                        )],
                    ));
                });
        });

    // Tile info display (bottom-left)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(280.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
        BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.8)),
        GameplayUIRoot,
        children![(
            Text::new("Hover over a tile"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            TileInfoDisplay,
        ),],
    ));

    // Sidebar with mode buttons
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(450.0),
                width: Val::Px(140.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.8)),
            GameplayUIRoot,
        ))
        .with_children(|sidebar| {
            use crate::ui::mode::{
                CityModeButton, DiplomacyModeButton, MapModeButton, MarketModeButton,
                TransportModeButton,
            };
            // Map Mode button
            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    MapModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Map"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
            // Transport Mode button
            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    TransportModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Transport"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
            // City Mode button
            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    CityModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("City"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
            // Market Mode button
            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    MarketModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Market"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
            // Diplomacy Mode button
            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    DiplomacyModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Diplomacy"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
        });
}

/// Show all Map UI elements (HUD, terminal, sidebar) and tilemap when entering Map mode
pub fn show_map_ui(
    mut ui_roots: Query<&mut Visibility, With<GameplayUIRoot>>,
    mut tilemaps: Query<&mut Visibility, (With<MapTilemap>, Without<GameplayUIRoot>)>,
) {
    for mut vis in ui_roots.iter_mut() {
        *vis = Visibility::Visible;
    }
    for mut vis in tilemaps.iter_mut() {
        *vis = Visibility::Visible;
    }
}

/// Hide all Map UI elements and tilemap when leaving Map mode (entering other modes)
pub fn hide_map_ui(
    mut ui_roots: Query<&mut Visibility, With<GameplayUIRoot>>,
    mut tilemaps: Query<&mut Visibility, (With<MapTilemap>, Without<GameplayUIRoot>)>,
) {
    for mut vis in ui_roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
    for mut vis in tilemaps.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
