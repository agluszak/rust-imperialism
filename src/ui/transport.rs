use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::nation::PlayerNation;
use crate::economy::production::ConnectedProduction;
use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::resources::{ALL_RESOURCES, ResourceType};
use crate::ui::logging::TerminalLogEvent;
use crate::ui::mode::GameMode;

#[derive(Component)]
pub struct TransportScreen;

/// Marker for a text element that displays info for a specific resource
#[derive(Component)]
pub struct ResourceDisplay(pub ResourceType);

#[derive(Resource, Default)]
pub struct TransportToolState {
    pub first: Option<TilePos>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct TransportSelectTile {
    pub pos: TilePos,
}

pub struct TransportUIPlugin;

impl Plugin for TransportUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransportToolState>()
            .add_message::<TransportSelectTile>()
            .add_systems(OnEnter(GameMode::Transport), setup_transport_screen)
            .add_systems(OnExit(GameMode::Transport), despawn_transport_screen)
            .add_systems(
                Update,
                (
                    handle_transport_selection,
                    update_transport_list, // Keep the list updated
                )
                    .run_if(in_state(GameMode::Transport)),
            );
    }
}

/// Update the text for each resource with the latest production data
fn update_transport_list(
    production: Res<ConnectedProduction>,
    player: Option<Res<PlayerNation>>,
    mut query: Query<(&mut Text, &ResourceDisplay)>,
) {
    let Some(player) = player else { return };

    for (mut text, display) in query.iter_mut() {
        let (count, total) = production
            .0
            .get(&player.0)
            .and_then(|p| p.get(&display.0))
            .copied()
            .unwrap_or((0, 0));

        // In this Bevy version, Text is a tuple struct: Text(String)
        text.0 = format!(
            "{:?}: {} improvements (producing {} units)",
            display.0, count, total
        );
    }
}

pub fn handle_transport_selection(
    mut ev: MessageReader<TransportSelectTile>,
    mut tool: ResMut<TransportToolState>,
    mut place_writer: MessageWriter<PlaceImprovement>,
    mut log: MessageWriter<TerminalLogEvent>,
) {
    for e in ev.read() {
        if let Some(a) = tool.first.take() {
            let b = e.pos;
            place_writer.write(PlaceImprovement {
                a,
                b,
                kind: ImprovementKind::Road,
                engineer: None,
            });
        } else {
            tool.first = Some(e.pos);
            log.write(TerminalLogEvent {
                message: format!("Selected tile ({}, {}) for road start", e.pos.x, e.pos.y),
            });
        }
    }
}

/// Create the transport screen UI when entering the transport game mode
fn setup_transport_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
            TransportScreen,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Transport Mode: Connected Production"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.95, 1.0)),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    margin: UiRect {
                        top: Val::Px(20.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|list| {
                    for &res_type in ALL_RESOURCES {
                        list.spawn((
                            Text::new(format!("{:?}:", res_type)),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.9)),
                            ResourceDisplay(res_type),
                        ));
                    }
                });

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
        });
}

/// Despawn the transport screen UI when exiting the transport game mode
fn despawn_transport_screen(
    mut commands: Commands,
    query: Query<Entity, With<TransportScreen>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}