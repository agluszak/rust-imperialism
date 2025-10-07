use bevy::prelude::*;

use super::button_style::*;
use crate::civilians::CivilianKind;
use crate::ui::mode::GameMode;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

/// Marker for hire civilian buttons
#[derive(Component)]
pub struct HireCivilianButton(pub CivilianKind);

/// Marker for building panels (dynamically created)
#[derive(Component)]
pub struct BuildingPanel;

/// Marker for production choice buttons
#[derive(Component)]
pub struct ProductionChoiceButton {
    pub building_entity: Entity,
    pub choice: crate::economy::production::ProductionChoice,
}

/// Marker for increase/decrease production buttons
#[derive(Component)]
pub struct AdjustProductionButton {
    pub building_entity: Entity,
    pub delta: i32, // +1 or -1
}

/// Message to hire a civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct HireCivilian {
    pub kind: CivilianKind,
}

/// Message to change production settings
#[derive(Message, Debug, Clone, Copy)]
pub struct ChangeProductionSettings {
    pub building_entity: Entity,
    pub new_choice: Option<crate::economy::production::ProductionChoice>,
    pub target_delta: Option<i32>,
}

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HireCivilian>()
            .add_message::<ChangeProductionSettings>()
            .add_systems(OnEnter(GameMode::City), ensure_city_screen_visible)
            .add_systems(OnExit(GameMode::City), hide_city_screen)
            .add_systems(
                Update,
                (
                    handle_hire_button_clicks,
                    spawn_hired_civilian,
                    handle_production_choice_buttons,
                    handle_adjust_production_buttons,
                    apply_production_settings_changes,
                    update_building_panels,
                )
                    .run_if(in_state(GameMode::City)),
            );
    }
}

pub fn ensure_city_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<CityScreen>>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    buildings_query: Query<(Entity, &crate::economy::Building, &crate::economy::production::ProductionSettings)>,
    stockpiles: Query<&crate::economy::Stockpile>,
) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    // Get player stockpile for display
    let (player_stockpile, player_entity) = if let Some(player) = &player_nation {
        (stockpiles.get(player.0).ok(), Some(player.0))
    } else {
        (None, None)
    };

    // Collect player's buildings
    let mut player_buildings = Vec::new();
    if let Some(player_ent) = player_entity {
        for (building_entity, building, settings) in buildings_query.iter() {
            if building_entity == player_ent || buildings_query.get(player_ent).is_err() {
                // This is the player's building (buildings are components on nation entity)
                player_buildings.push((building_entity, building, settings));
            }
        }
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
                    BackgroundColor(NORMAL_BUTTON),
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

            // Stockpile display
            if let Some(stockpile) = player_stockpile {
                parent.spawn((
                    Text::new(format!(
                        "Warehouse: Wool: {}, Cotton: {}, Cloth: {}",
                        stockpile.get(crate::economy::goods::Good::Wool),
                        stockpile.get(crate::economy::goods::Good::Cotton),
                        stockpile.get(crate::economy::goods::Good::Cloth)
                    )),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.9, 0.8)),
                ));
            }

            // Buildings panel - dynamically created
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.15, 0.9)),
                    BuildingPanel,
                ))
                .with_children(|buildings_container| {
                    use crate::economy::production::{BuildingKind, ProductionChoice};
                    use crate::economy::goods::Good;

                    if player_buildings.is_empty() {
                        buildings_container.spawn((
                            Text::new("No buildings yet"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        ));
                    } else {
                        for (building_entity, building, settings) in player_buildings.iter() {
                            // Inline building UI creation
                            let (name, input_desc, output_desc) = match building.kind {
                                BuildingKind::TextileMill => {
                                    let input_choice = match settings.choice {
                                        ProductionChoice::UseCotton => "2× Cotton",
                                        ProductionChoice::UseWool => "2× Wool",
                                    };
                                    ("Textile Mill", input_choice, "1× Cloth")
                                }
                            };

                            buildings_container
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::all(Val::Px(12.0)),
                                        row_gap: Val::Px(8.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.15, 0.18, 0.9)),
                                ))
                                .with_children(|row| {
                                    row.spawn((
                                        Text::new(format!("{}  —  Capacity: {}", name, building.capacity)),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.95, 0.95, 1.0)),
                                    ));

                                    row.spawn((
                                        Text::new(format!("{}  →  {}", input_desc, output_desc)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                    ));

                                    if let Some(stockpile) = player_stockpile {
                                        let input_good = match settings.choice {
                                            ProductionChoice::UseCotton => Good::Cotton,
                                            ProductionChoice::UseWool => Good::Wool,
                                        };
                                        let available = stockpile.get(input_good);
                                        row.spawn((
                                            Text::new(format!(
                                                "Target: {} | Available {}: {} (need {})",
                                                settings.target_output,
                                                input_good,
                                                available,
                                                settings.target_output * 2
                                            )),
                                            TextFont {
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                        ));
                                    }

                                    // Production controls
                                    row.spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(8.0),
                                        ..default()
                                    })
                                    .with_children(|controls| {
                                        // Choice buttons
                                        if building.kind == BuildingKind::TextileMill {
                                            controls
                                                .spawn((
                                                    Button,
                                                    Node {
                                                        padding: UiRect::all(Val::Px(6.0)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(if settings.choice == ProductionChoice::UseCotton {
                                                        PRESSED_BUTTON
                                                    } else {
                                                        NORMAL_BUTTON
                                                    }),
                                                    ProductionChoiceButton {
                                                        building_entity: *building_entity,
                                                        choice: ProductionChoice::UseCotton,
                                                    },
                                                ))
                                                .with_children(|b| {
                                                    b.spawn((
                                                        Text::new("Use Cotton"),
                                                        TextFont {
                                                            font_size: 13.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                                    ));
                                                });

                                            controls
                                                .spawn((
                                                    Button,
                                                    Node {
                                                        padding: UiRect::all(Val::Px(6.0)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(if settings.choice == ProductionChoice::UseWool {
                                                        PRESSED_BUTTON
                                                    } else {
                                                        NORMAL_BUTTON
                                                    }),
                                                    ProductionChoiceButton {
                                                        building_entity: *building_entity,
                                                        choice: ProductionChoice::UseWool,
                                                    },
                                                ))
                                                .with_children(|b| {
                                                    b.spawn((
                                                        Text::new("Use Wool"),
                                                        TextFont {
                                                            font_size: 13.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                                    ));
                                                });
                                        }

                                        // Adjust buttons
                                        controls
                                            .spawn((
                                                Button,
                                                Node {
                                                    padding: UiRect::all(Val::Px(6.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(NORMAL_ACCENT),
                                                AccentButton,
                                                AdjustProductionButton {
                                                    building_entity: *building_entity,
                                                    delta: -1,
                                                },
                                            ))
                                            .with_children(|b| {
                                                b.spawn((
                                                    Text::new(" − "),
                                                    TextFont {
                                                        font_size: 16.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                                                ));
                                            });

                                        controls
                                            .spawn((
                                                Button,
                                                Node {
                                                    padding: UiRect::all(Val::Px(6.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(NORMAL_ACCENT),
                                                AccentButton,
                                                AdjustProductionButton {
                                                    building_entity: *building_entity,
                                                    delta: 1,
                                                },
                                            ))
                                            .with_children(|b| {
                                                b.spawn((
                                                    Text::new(" + "),
                                                    TextFont {
                                                        font_size: 16.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                                                ));
                                            });
                                    });
                                });
                        }
                    }
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
                            grid_template_columns: vec![RepeatedGridTrack::auto(3)],
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
                                    BackgroundColor(NORMAL_ACCENT),
                                    AccentButton,
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
                message: format!(
                    "Not enough money to hire {:?} (need ${}, have ${})",
                    event.kind, cost, treasury.0
                ),
            });
            continue;
        }

        // Find unoccupied tile near capital
        let spawn_pos = find_unoccupied_tile_near(capital.0, &tile_storage_query, &civilians);

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
            if let Some(neighbor_pos) = neighbor_hex.to_tile_pos()
                && tile_storage_query
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

    None
}

/// Check if a tile is occupied by any civilian
fn is_tile_occupied(
    pos: bevy_ecs_tilemap::prelude::TilePos,
    civilians: &Query<&crate::civilians::Civilian>,
) -> bool {
    civilians.iter().any(|c| c.position == pos)
}

/// Handle production choice button clicks
fn handle_production_choice_buttons(
    interactions: Query<(&Interaction, &ProductionChoiceButton), Changed<Interaction>>,
    mut change_writer: MessageWriter<ChangeProductionSettings>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            info!("Production choice button clicked: {:?}", button.choice);
            change_writer.write(ChangeProductionSettings {
                building_entity: button.building_entity,
                new_choice: Some(button.choice),
                target_delta: None,
            });
        }
    }
}

/// Handle adjust production button clicks
fn handle_adjust_production_buttons(
    interactions: Query<(&Interaction, &AdjustProductionButton), Changed<Interaction>>,
    mut change_writer: MessageWriter<ChangeProductionSettings>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            info!("Adjust production button clicked: delta {}", button.delta);
            change_writer.write(ChangeProductionSettings {
                building_entity: button.building_entity,
                new_choice: None,
                target_delta: Some(button.delta),
            });
        }
    }
}

/// Apply production settings changes
fn apply_production_settings_changes(
    mut change_events: MessageReader<ChangeProductionSettings>,
    mut settings_query: Query<&mut crate::economy::production::ProductionSettings>,
    buildings_query: Query<&crate::economy::Building>,
) {
    for event in change_events.read() {
        if let Ok(mut settings) = settings_query.get_mut(event.building_entity) {
            // Apply choice change
            if let Some(new_choice) = event.new_choice {
                settings.choice = new_choice;
                info!("Changed production choice to {:?}", new_choice);
            }

            // Apply target delta
            if let Some(delta) = event.target_delta {
                let new_target = (settings.target_output as i32 + delta).max(0) as u32;

                // Cap by building capacity
                if let Ok(building) = buildings_query.get(event.building_entity) {
                    settings.target_output = new_target.min(building.capacity);
                } else {
                    settings.target_output = new_target;
                }

                info!("Adjusted production target to {}", settings.target_output);
            }
        }
    }
}

/// Update building panels when data changes (for dynamic updates)
fn update_building_panels(
    // This is a placeholder for now - we'll implement dynamic updates if needed
    // Currently the UI is rebuilt when entering City mode
) {
    // For now, UI only updates when entering/exiting City mode
    // Could add dynamic updates here in the future
}
