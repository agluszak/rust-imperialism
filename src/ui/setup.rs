use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::ui::components::*;

pub fn setup_ui(mut commands: Commands) {
    // Create UI root for status display
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // Turn display
            parent.spawn((
                Text::new("Turn: 1 - Player Turn"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TurnDisplay,
            ));

            // Hero status display
            parent.spawn((
                Text::new("Hero: HP 10/10, MP 3/3, Kills: 0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                HeroStatusDisplay,
            ));
        });

    // Create terminal window
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(420.0), // Make room for scrollbar
                height: Val::Px(300.0),
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(5.0)),
                flex_direction: FlexDirection::Row, // Changed to row to accommodate scrollbar
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            TerminalWindow,
        ))
        .with_children(|parent| {
            // Content area container
            parent
                .spawn((Node {
                    width: Val::Percent(95.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },))
                .with_children(|content| {
                    // Terminal header
                    content.spawn((
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

                    // Scrollable content area using native Bevy scrolling
                    content
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition::default(),
                            ScrollableTerminal,
                            RelativeCursorPosition::default(),
                        ))
                        .with_children(|scrollable| {
                            scrollable.spawn((
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
                            ));
                        });
                });

            // Scrollbar
            parent
                .spawn((
                    Node {
                        width: Val::Percent(5.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                    RelativeCursorPosition::default(),
                    ScrollbarTrack,
                ))
                .with_children(|track| {
                    // Scrollbar thumb
                    track.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(20.0), // Initial thumb size
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                        ScrollbarThumb,
                        RelativeCursorPosition::default(),
                    ));
                });
        });
}
