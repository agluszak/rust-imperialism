use bevy::prelude::*;

use crate::ui::mode::GameMode;
use crate::civilians::CivilianKind;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

/// Marker for hire civilian buttons
#[derive(Component)]
pub struct HireCivilianButton(pub CivilianKind);

/// Message to hire a civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct HireCivilian {
    pub kind: CivilianKind,
}

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HireCivilian>()
            .add_systems(OnEnter(GameMode::City), ensure_city_screen_visible)
            .add_systems(OnExit(GameMode::City), hide_city_screen)
            .add_systems(
                Update,
                (handle_hire_button_clicks, spawn_hired_civilian).run_if(in_state(GameMode::City)),
            );
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

            // Civilian Hiring Panel
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.15, 0.12, 0.9)),
                ))
                .with_children(|hiring| {
                    hiring.spawn((
                        Text::new("Hire Civilians"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ));

                    // Grid of hire buttons
                    hiring
                        .spawn(Node {
                            display: Display::Grid,
                            grid_template_columns: vec![
                                RepeatedGridTrack::auto(3),
                            ],
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|grid| {
                            let civilians = [
                                (CivilianKind::Engineer, "Engineer", "$200"),
                                (CivilianKind::Prospector, "Prospector", "$150"),
                                (CivilianKind::Farmer, "Farmer", "$100"),
                                (CivilianKind::Rancher, "Rancher", "$100"),
                                (CivilianKind::Forester, "Forester", "$100"),
                                (CivilianKind::Miner, "Miner", "$120"),
                                (CivilianKind::Driller, "Driller", "$120"),
                                (CivilianKind::Developer, "Developer", "$180"),
                            ];

                            for (kind, name, cost) in civilians {
                                grid.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::all(Val::Px(10.0)),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.25, 0.2, 1.0)),
                                    HireCivilianButton(kind),
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new(name),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.95, 0.95, 1.0)),
                                    ));
                                    b.spawn((
                                        Text::new(cost),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.9, 0.8)),
                                    ));
                                });
                            }
                        });
                });
        });
}

pub fn hide_city_screen(mut roots: Query<&mut Visibility, With<CityScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

/// Handle clicks on hire civilian buttons
fn handle_hire_button_clicks(
    interactions: Query<(&Interaction, &HireCivilianButton), Changed<Interaction>>,
    mut hire_writer: MessageWriter<HireCivilian>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            info!("Hire {:?} button clicked", button.0);
            hire_writer.write(HireCivilian { kind: button.0 });
        }
    }
}

/// Spawn hired civilian near capital
fn spawn_hired_civilian(
    mut commands: Commands,
    mut hire_events: MessageReader<HireCivilian>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    nations: Query<&crate::economy::Capital>,
    mut treasuries: Query<&mut crate::economy::Treasury>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: Query<&crate::civilians::Civilian>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for event in hire_events.read() {
        let Some(player) = &player_nation else {
            continue;
        };

        // Get capital position
        let Ok(capital) = nations.get(player.0) else {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: "Cannot hire: no capital found".to_string(),
            });
            continue;
        };

        // Determine cost based on civilian type
        let cost = match event.kind {
            CivilianKind::Engineer => 200,
            CivilianKind::Prospector => 150,
            CivilianKind::Developer => 180,
            CivilianKind::Miner | CivilianKind::Driller => 120,
            _ => 100,
        };

        // Check if player can afford
        let Ok(mut treasury) = treasuries.get_mut(player.0) else {
            continue;
        };

        if treasury.0 < cost {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!("Not enough money to hire {:?} (need ${}, have ${})", event.kind, cost, treasury.0),
            });
            continue;
        }

        // Find unoccupied tile near capital
        let spawn_pos = find_unoccupied_tile_near(
            capital.0,
            &tile_storage_query,
            &civilians,
        );

        let Some(spawn_pos) = spawn_pos else {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: "No unoccupied tiles near capital to spawn civilian".to_string(),
            });
            continue;
        };

        // Deduct cost
        treasury.0 -= cost;

        // Spawn civilian
        commands.spawn(crate::civilians::Civilian {
            kind: event.kind,
            position: spawn_pos,
            owner: player.0,
            selected: false,
            has_moved: false,
        });

        log_events.write(crate::ui::logging::TerminalLogEvent {
            message: format!(
                "Hired {:?} for ${} at ({}, {})",
                event.kind, cost, spawn_pos.x, spawn_pos.y
            ),
        });
    }
}

/// Find an unoccupied tile near the given position
fn find_unoccupied_tile_near(
    center: bevy_ecs_tilemap::prelude::TilePos,
    tile_storage_query: &Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    civilians: &Query<&crate::civilians::Civilian>,
) -> Option<bevy_ecs_tilemap::prelude::TilePos> {
    use crate::tile_pos::{HexExt, TilePosExt};

    let center_hex = center.to_hex();

    // Check center first
    if !is_tile_occupied(center, civilians) {
        return Some(center);
    }

    // Check neighbors in expanding rings
    for radius in 1..=3 {
        for neighbor_hex in center_hex.ring(radius) {
            if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                if tile_storage_query
                    .iter()
                    .next()
                    .and_then(|storage| storage.get(&neighbor_pos))
                    .is_some()
                    && !is_tile_occupied(neighbor_pos, civilians)
                {
                    return Some(neighbor_pos);
                }
            }
        }
    }

    None
}

/// Check if a tile is occupied by any civilian
fn is_tile_occupied(
    pos: bevy_ecs_tilemap::prelude::TilePos,
    civilians: &Query<&crate::civilians::Civilian>,
) -> bool {
    civilians.iter().any(|c| c.position == pos)
}
