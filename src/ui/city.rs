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

/// Marker for recruit workers button
#[derive(Component)]
pub struct RecruitWorkersButton {
    pub count: u32,
}

/// Marker for train worker button
#[derive(Component)]
pub struct TrainWorkerButton {
    pub from_skill: crate::economy::WorkerSkill,
}

/// Marker for the workforce panel
#[derive(Component)]
pub struct WorkforcePanel;

/// Marker for workforce counts text (updates dynamically)
#[derive(Component)]
pub struct WorkforceCountsText;

/// Marker for available labor text (updates dynamically)
#[derive(Component)]
pub struct AvailableLaborText;

/// Marker for stockpile food text
#[derive(Component)]
pub struct StockpileFoodText;

/// Marker for stockpile materials text
#[derive(Component)]
pub struct StockpileMaterialsText;

/// Marker for stockpile goods text
#[derive(Component)]
pub struct StockpileGoodsText;

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HireCivilian>()
            .add_message::<ChangeProductionSettings>()
            .add_message::<crate::economy::RecruitWorkers>()
            .add_message::<crate::economy::TrainWorker>()
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
                    handle_recruit_workers_buttons,
                    handle_train_worker_buttons,
                    update_building_panels,
                    update_workforce_panel,
                    update_stockpile_display,
                )
                    .run_if(in_state(GameMode::City)),
            );
    }
}

pub fn ensure_city_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<CityScreen>>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    buildings_query: Query<(
        Entity,
        &crate::economy::Building,
        &crate::economy::production::ProductionSettings,
    )>,
    stockpiles: Query<&crate::economy::Stockpile>,
    workforces: Query<&crate::economy::Workforce>,
    recruitment_queues: Query<&crate::economy::RecruitmentQueue>,
    training_queues: Query<&crate::economy::TrainingQueue>,
    provinces: Query<&crate::province::Province>,
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
                use crate::economy::goods::Good;

                // Show food resources (available/total)
                parent.spawn((
                    Text::new(format!(
                        "Food: Grain: {}/{}, Fruit: {}/{}, Livestock: {}/{}, Canned: {}/{}",
                        stockpile.get_available(Good::Grain), stockpile.get(Good::Grain),
                        stockpile.get_available(Good::Fruit), stockpile.get(Good::Fruit),
                        stockpile.get_available(Good::Livestock), stockpile.get(Good::Livestock),
                        stockpile.get_available(Good::CannedFood), stockpile.get(Good::CannedFood)
                    )),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.9, 0.8)),
                    StockpileFoodText, // Marker for dynamic updates
                ));

                // Show materials and goods (available/total)
                parent.spawn((
                    Text::new(format!(
                        "Materials: Wool: {}/{}, Cotton: {}/{}, Fabric: {}/{}, Paper: {}/{}",
                        stockpile.get_available(Good::Wool), stockpile.get(Good::Wool),
                        stockpile.get_available(Good::Cotton), stockpile.get(Good::Cotton),
                        stockpile.get_available(Good::Fabric), stockpile.get(Good::Fabric),
                        stockpile.get_available(Good::Paper), stockpile.get(Good::Paper)
                    )),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.9, 0.8)),
                    StockpileMaterialsText, // Marker for dynamic updates
                ));

                // Show finished goods (available/total)
                parent.spawn((
                    Text::new(format!(
                        "Goods: Clothing: {}/{}, Furniture: {}/{}",
                        stockpile.get_available(Good::Clothing), stockpile.get(Good::Clothing),
                        stockpile.get_available(Good::Furniture), stockpile.get(Good::Furniture)
                    )),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.9, 0.8)),
                    StockpileGoodsText, // Marker for dynamic updates
                ));
            }

            // Workforce panel
            if let Some(player_ent) = player_entity
                && let Ok(workforce) = workforces.get(player_ent) {
                    let province_count = provinces.iter().filter(|p| p.owner == Some(player_ent)).count();
                    let recruit_cap = crate::economy::workforce::calculate_recruitment_cap(
                        province_count as u32,
                        false, // TODO: Check for upgrade
                    );

                    let recruitment_queue = recruitment_queues.get(player_ent).ok();
                    let training_queue = training_queues.get(player_ent).ok();

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.12, 0.18, 0.9)),
                            WorkforcePanel,
                        ))
                        .with_children(|panel| {
                            // Title
                            panel.spawn((
                                Text::new("Workforce"),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.95, 1.0)),
                            ));

                            // Worker counts
                            let untrained = workforce.untrained_count();
                            let trained = workforce.trained_count();
                            let expert = workforce.expert_count();
                            let available_labor = workforce.available_labor();

                            panel.spawn((
                                Text::new(format!(
                                    "Untrained: {} (1 labor) | Trained: {} (2 labor) | Expert: {} (4 labor)",
                                    untrained, trained, expert
                                )),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                WorkforceCountsText, // Marker for dynamic updates
                            ));

                            panel.spawn((
                                Text::new(format!("Available Labor: {}", available_labor)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.9, 0.7)),
                                AvailableLaborText, // Marker for dynamic updates
                            ));

                            // Show queued orders
                            if let Some(recruitment_queue) = recruitment_queue
                                && recruitment_queue.queued > 0 {
                                    panel.spawn((
                                        Text::new(format!("Queued recruitment: {} workers (will hire next turn)", recruitment_queue.queued)),
                                        TextFont {
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.9, 0.5)),
                                    ));
                                }

                            if let Some(training_queue) = training_queue
                                && !training_queue.orders.is_empty() {
                                    let total = training_queue.total_queued();
                                    panel.spawn((
                                        Text::new(format!("Queued training: {} workers (will train next turn)", total)),
                                        TextFont {
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.9, 0.5)),
                                    ));
                                }

                            // Recruitment section (Capitol)
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    padding: UiRect::top(Val::Px(8.0)),
                                    ..default()
                                })
                                .with_children(|row| {
                                    row.spawn((
                                        Text::new(format!("Recruit (cap: {}): ", recruit_cap)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.85, 0.85, 0.9)),
                                    ));

                                    // Recruit 1 button
                                    row.spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::all(Val::Px(6.0)),
                                            ..default()
                                        },
                                        BackgroundColor(NORMAL_BUTTON),
                                        RecruitWorkersButton { count: 1 },
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("+1"),
                                            TextFont {
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                        ));
                                    });

                                    // Recruit max button
                                    row.spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::all(Val::Px(6.0)),
                                            ..default()
                                        },
                                        BackgroundColor(NORMAL_BUTTON),
                                        RecruitWorkersButton { count: recruit_cap },
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new(format!("+{}", recruit_cap)),
                                            TextFont {
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                        ));
                                    });

                                    row.spawn((
                                        Text::new("(needs: Canned Food, Clothing, Furniture)"),
                                        TextFont {
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.7, 0.7, 0.75)),
                                    ));
                                });

                            // Training section (Trade School)
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    padding: UiRect::top(Val::Px(4.0)),
                                    ..default()
                                })
                                .with_children(|row| {
                                    row.spawn((
                                        Text::new("Train: "),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.85, 0.85, 0.9)),
                                    ));

                                    // Train Untrained -> Trained
                                    if untrained > 0 {
                                        row.spawn((
                                            Button,
                                            Node {
                                                padding: UiRect::all(Val::Px(6.0)),
                                                ..default()
                                            },
                                            BackgroundColor(NORMAL_BUTTON),
                                            TrainWorkerButton {
                                                from_skill: crate::economy::WorkerSkill::Untrained,
                                            },
                                        ))
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new("Untrained->Trained"),
                                                TextFont {
                                                    font_size: 13.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                            ));
                                        });
                                    }

                                    // Train Trained -> Expert
                                    if trained > 0 {
                                        row.spawn((
                                            Button,
                                            Node {
                                                padding: UiRect::all(Val::Px(6.0)),
                                                ..default()
                                            },
                                            BackgroundColor(NORMAL_BUTTON),
                                            TrainWorkerButton {
                                                from_skill: crate::economy::WorkerSkill::Trained,
                                            },
                                        ))
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new("Trained->Expert"),
                                                TextFont {
                                                    font_size: 13.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                            ));
                                        });
                                    }

                                    row.spawn((
                                        Text::new("(costs: 1 Paper, $100)"),
                                        TextFont {
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.7, 0.7, 0.75)),
                                    ));
                                });
                        });
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
                                        _ => "2× (input)",
                                    };
                                    ("Textile Mill", input_choice, "1× Fabric")
                                }
                                BuildingKind::LumberMill => {
                                    let output_choice = match settings.choice {
                                        ProductionChoice::MakeLumber => "1× Lumber",
                                        ProductionChoice::MakePaper => "1× Paper",
                                        _ => "1× (output)",
                                    };
                                    ("Lumber Mill", "2× Timber", output_choice)
                                }
                                BuildingKind::SteelMill => {
                                    ("Steel Mill", "1× Iron + 1× Coal", "1× Steel")
                                }
                                BuildingKind::FoodProcessingCenter => {
                                    let input_desc = match settings.choice {
                                        ProductionChoice::UseLivestock => "2× Grain + 1× Fruit + 1× Livestock",
                                        ProductionChoice::UseFish => "2× Grain + 1× Fruit + 1× Fish",
                                        _ => "2× Grain + 1× Fruit + 1× (meat)",
                                    };
                                    ("Food Processing", input_desc, "2× Canned Food")
                                }
                                BuildingKind::Capitol => {
                                    ("Capitol", "Recruits workers", "")
                                }
                                BuildingKind::TradeSchool => {
                                    ("Trade School", "Trains workers", "")
                                }
                                BuildingKind::PowerPlant => {
                                    ("Power Plant", "Fuel", "Labor")
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
                                        Text::new(format!("{}  ->  {}", input_desc, output_desc)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                    ));

                                    if let Some(stockpile) = player_stockpile {
                                        // Only show stockpile info for production buildings
                                        let status_text = match building.kind {
                                            BuildingKind::TextileMill => {
                                                let input_good = match settings.choice {
                                                    ProductionChoice::UseCotton => Good::Cotton,
                                                    ProductionChoice::UseWool => Good::Wool,
                                                    _ => Good::Cotton,
                                                };
                                                let available = stockpile.get(input_good);
                                                Some(format!(
                                                    "Target: {} | Available {}: {} (need {})",
                                                    settings.target_output,
                                                    input_good,
                                                    available,
                                                    settings.target_output * 2
                                                ))
                                            }
                                            BuildingKind::LumberMill => {
                                                let available = stockpile.get(Good::Timber);
                                                Some(format!(
                                                    "Target: {} | Available Timber: {} (need {})",
                                                    settings.target_output,
                                                    available,
                                                    settings.target_output * 2
                                                ))
                                            }
                                            BuildingKind::SteelMill => {
                                                let iron = stockpile.get(Good::Iron);
                                                let coal = stockpile.get(Good::Coal);
                                                Some(format!(
                                                    "Target: {} | Available Iron: {}, Coal: {} (need {} each)",
                                                    settings.target_output, iron, coal, settings.target_output
                                                ))
                                            }
                                            BuildingKind::FoodProcessingCenter => {
                                                let grain = stockpile.get(Good::Grain);
                                                let fruit = stockpile.get(Good::Fruit);
                                                let meat = match settings.choice {
                                                    ProductionChoice::UseLivestock => stockpile.get(Good::Livestock),
                                                    ProductionChoice::UseFish => stockpile.get(Good::Fish),
                                                    _ => 0,
                                                };
                                                let meat_name = match settings.choice {
                                                    ProductionChoice::UseLivestock => "Livestock",
                                                    ProductionChoice::UseFish => "Fish",
                                                    _ => "meat",
                                                };
                                                Some(format!(
                                                    "Target: {} | Grain: {}, Fruit: {}, {}: {}",
                                                    settings.target_output, grain, fruit, meat_name, meat
                                                ))
                                            }
                                            _ => None, // No status for non-production buildings
                                        };

                                        if let Some(text) = status_text {
                                            row.spawn((
                                                Text::new(text),
                                                TextFont {
                                                    font_size: 13.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                            ));
                                        }
                                    }

                                    // Production controls
                                    row.spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(8.0),
                                        ..default()
                                    })
                                    .with_children(|controls| {
                                        // Choice buttons (only for buildings with choices)
                                        match building.kind {
                                            BuildingKind::TextileMill => {
                                                // Cotton vs Wool
                                                for (choice, label) in [
                                                    (ProductionChoice::UseCotton, "Use Cotton"),
                                                    (ProductionChoice::UseWool, "Use Wool"),
                                                ] {
                                                    controls
                                                        .spawn((
                                                            Button,
                                                            Node {
                                                                padding: UiRect::all(Val::Px(6.0)),
                                                                ..default()
                                                            },
                                                            BackgroundColor(if settings.choice == choice {
                                                                PRESSED_BUTTON
                                                            } else {
                                                                NORMAL_BUTTON
                                                            }),
                                                            ProductionChoiceButton {
                                                                building_entity: *building_entity,
                                                                choice,
                                                            },
                                                        ))
                                                        .with_children(|b| {
                                                            b.spawn((
                                                                Text::new(label),
                                                                TextFont {
                                                                    font_size: 13.0,
                                                                    ..default()
                                                                },
                                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                                            ));
                                                        });
                                                }
                                            }
                                            BuildingKind::LumberMill => {
                                                // Lumber vs Paper
                                                for (choice, label) in [
                                                    (ProductionChoice::MakeLumber, "Make Lumber"),
                                                    (ProductionChoice::MakePaper, "Make Paper"),
                                                ] {
                                                    controls
                                                        .spawn((
                                                            Button,
                                                            Node {
                                                                padding: UiRect::all(Val::Px(6.0)),
                                                                ..default()
                                                            },
                                                            BackgroundColor(if settings.choice == choice {
                                                                PRESSED_BUTTON
                                                            } else {
                                                                NORMAL_BUTTON
                                                            }),
                                                            ProductionChoiceButton {
                                                                building_entity: *building_entity,
                                                                choice,
                                                            },
                                                        ))
                                                        .with_children(|b| {
                                                            b.spawn((
                                                                Text::new(label),
                                                                TextFont {
                                                                    font_size: 13.0,
                                                                    ..default()
                                                                },
                                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                                            ));
                                                        });
                                                }
                                            }
                                            BuildingKind::FoodProcessingCenter => {
                                                // Livestock vs Fish
                                                for (choice, label) in [
                                                    (ProductionChoice::UseLivestock, "Use Livestock"),
                                                    (ProductionChoice::UseFish, "Use Fish"),
                                                ] {
                                                    controls
                                                        .spawn((
                                                            Button,
                                                            Node {
                                                                padding: UiRect::all(Val::Px(6.0)),
                                                                ..default()
                                                            },
                                                            BackgroundColor(if settings.choice == choice {
                                                                PRESSED_BUTTON
                                                            } else {
                                                                NORMAL_BUTTON
                                                            }),
                                                            ProductionChoiceButton {
                                                                building_entity: *building_entity,
                                                                choice,
                                                            },
                                                        ))
                                                        .with_children(|b| {
                                                            b.spawn((
                                                                Text::new(label),
                                                                TextFont {
                                                                    font_size: 13.0,
                                                                    ..default()
                                                                },
                                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                                            ));
                                                        });
                                                }
                                            }
                                            // No choice buttons for SteelMill or worker buildings
                                            _ => {}
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

/// Handle recruit workers button clicks (Input Layer)
fn handle_recruit_workers_buttons(
    interactions: Query<(&Interaction, &RecruitWorkersButton), Changed<Interaction>>,
    mut writer: MessageWriter<crate::economy::RecruitWorkers>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    buttons: Query<Entity, With<RecruitWorkersButton>>,
) {
    // Debug: check if buttons exist
    let button_count = buttons.iter().count();
    if button_count > 0 {
        trace!("Found {} recruit buttons in scene", button_count);
    }

    let Some(player_nation) = player_nation else {
        warn!("No player nation found for recruitment");
        return;
    };

    for (interaction, button) in interactions.iter() {
        debug!("Recruit button interaction: {:?}", interaction);
        if *interaction == Interaction::Pressed {
            info!("Recruit {} workers button clicked", button.count);
            writer.write(crate::economy::RecruitWorkers {
                nation: player_nation.0,
                count: button.count,
            });
        }
    }
}

/// Handle train worker button clicks (Input Layer)
fn handle_train_worker_buttons(
    interactions: Query<(&Interaction, &TrainWorkerButton), Changed<Interaction>>,
    mut writer: MessageWriter<crate::economy::TrainWorker>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    buttons: Query<Entity, With<TrainWorkerButton>>,
) {
    // Debug: check if buttons exist
    let button_count = buttons.iter().count();
    if button_count > 0 {
        trace!("Found {} train buttons in scene", button_count);
    }

    let Some(player_nation) = player_nation else {
        warn!("No player nation found for training");
        return;
    };

    for (interaction, button) in interactions.iter() {
        debug!("Train button interaction: {:?}", interaction);
        if *interaction == Interaction::Pressed {
            info!("Train worker button clicked: {:?}", button.from_skill);
            writer.write(crate::economy::TrainWorker {
                nation: player_nation.0,
                from_skill: button.from_skill,
            });
        }
    }
}

/// Update workforce panel when data changes (Rendering Layer)
/// Updates workforce panel text when workforce data changes
fn update_workforce_panel(
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    workforces: Query<&crate::economy::Workforce, Changed<crate::economy::Workforce>>,
    mut worker_counts_text: Query<
        &mut Text,
        (With<WorkforceCountsText>, Without<AvailableLaborText>),
    >,
    mut labor_text: Query<&mut Text, (With<AvailableLaborText>, Without<WorkforceCountsText>)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    // Check if player's workforce changed
    if let Ok(workforce) = workforces.get(player.0) {
        let untrained = workforce.untrained_count();
        let trained = workforce.trained_count();
        let expert = workforce.expert_count();
        let available_labor = workforce.available_labor();

        // Update worker counts text
        for mut text in worker_counts_text.iter_mut() {
            text.0 = format!(
                "Untrained: {} (1 labor) | Trained: {} (2 labor) | Expert: {} (4 labor)",
                untrained, trained, expert
            );
        }

        // Update available labor text
        for mut text in labor_text.iter_mut() {
            text.0 = format!("Available Labor: {}", available_labor);
        }
    }
}

/// Update building panels when data changes (for dynamic updates)
fn update_building_panels(// This is a placeholder for now - we'll implement dynamic updates if needed
    // Currently the UI is rebuilt when entering City mode
) {
    // For now, UI only updates when entering/exiting City mode
    // Could add dynamic updates here in the future
}

/// Updates stockpile display when stockpile data changes
fn update_stockpile_display(
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    stockpiles: Query<&crate::economy::Stockpile, Changed<crate::economy::Stockpile>>,
    mut food_text: Query<
        &mut Text,
        (
            With<StockpileFoodText>,
            Without<StockpileMaterialsText>,
            Without<StockpileGoodsText>,
        ),
    >,
    mut materials_text: Query<
        &mut Text,
        (
            With<StockpileMaterialsText>,
            Without<StockpileFoodText>,
            Without<StockpileGoodsText>,
        ),
    >,
    mut goods_text: Query<
        &mut Text,
        (
            With<StockpileGoodsText>,
            Without<StockpileFoodText>,
            Without<StockpileMaterialsText>,
        ),
    >,
) {
    use crate::economy::goods::Good;

    let Some(player) = player_nation else {
        return;
    };

    // Check if player's stockpile changed
    if let Ok(stockpile) = stockpiles.get(player.0) {
        // Update food text (available/total)
        for mut text in food_text.iter_mut() {
            text.0 = format!(
                "Food: Grain: {}/{}, Fruit: {}/{}, Livestock: {}/{}, Canned: {}/{}",
                stockpile.get_available(Good::Grain),
                stockpile.get(Good::Grain),
                stockpile.get_available(Good::Fruit),
                stockpile.get(Good::Fruit),
                stockpile.get_available(Good::Livestock),
                stockpile.get(Good::Livestock),
                stockpile.get_available(Good::CannedFood),
                stockpile.get(Good::CannedFood)
            );
        }

        // Update materials text (available/total)
        for mut text in materials_text.iter_mut() {
            text.0 = format!(
                "Materials: Wool: {}/{}, Cotton: {}/{}, Fabric: {}/{}, Paper: {}/{}",
                stockpile.get_available(Good::Wool),
                stockpile.get(Good::Wool),
                stockpile.get_available(Good::Cotton),
                stockpile.get(Good::Cotton),
                stockpile.get_available(Good::Fabric),
                stockpile.get(Good::Fabric),
                stockpile.get_available(Good::Paper),
                stockpile.get(Good::Paper)
            );
        }

        // Update goods text (available/total)
        for mut text in goods_text.iter_mut() {
            text.0 = format!(
                "Goods: Clothing: {}/{}, Furniture: {}/{}",
                stockpile.get_available(Good::Clothing),
                stockpile.get(Good::Clothing),
                stockpile.get_available(Good::Furniture),
                stockpile.get(Good::Furniture)
            );
        }
    }
}
