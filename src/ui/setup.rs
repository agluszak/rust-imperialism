use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::ui_widgets::{ControlOrientation, CoreScrollbarThumb, Scrollbar};

use crate::ui::components::{
    HeroStatusDisplay, MonsterCountDisplay, ScrollableTerminal, TerminalOutput, TerminalWindow,
    TurnDisplay,
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
        children![
            // Turn info section
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                children![(
                    Text::new("Turn: 1 - Player Turn"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    TurnDisplay,
                )],
            ),
            // Hero status section
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                children![
                    // Hero label
                    (
                        Text::new("HERO"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                    ),
                    // Hero stats
                    (
                        Text::new("HP 10/10, AP 6/6, Kills: 0"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 0.7)),
                        HeroStatusDisplay,
                    ),
                ],
            ),
            // Monsters section
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                children![
                    // Monsters label
                    (
                        Text::new("ENEMIES"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                    ),
                    // Monster count
                    (
                        Text::new("Monsters: 0"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.6, 0.6)),
                        MonsterCountDisplay,
                    ),
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
}
