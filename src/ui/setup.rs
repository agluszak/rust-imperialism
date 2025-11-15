use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::Button;

use crate::ui::button_style::*;
use crate::ui::components::{
    CalendarDisplay, GameplayUIRoot, TileInfoDisplay, TreasuryDisplay, TurnDisplay,
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
                right: Val::Px(10.0),
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
            use crate::ui::mode::{GameMode, switch_to_mode};
            // Transport Mode button
            sidebar.spawn((
                Button,
                OldButton,
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::Transport),
                children![(
                    Text::new("Transport"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ));
            // City Mode button
            sidebar.spawn((
                Button,
                OldButton,
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::City),
                children![(
                    Text::new("City"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ));
            // Market Mode button
            sidebar.spawn((
                Button,
                OldButton,
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::Market),
                children![(
                    Text::new("Market"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ));
            // Diplomacy Mode button
            sidebar.spawn((
                Button,
                OldButton,
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::Diplomacy),
                children![(
                    Text::new("Diplomacy"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ));
        });
}
