use bevy::prelude::*;

use crate::economy::production::{BuildingKind, ProductionChoice, ProductionSettings};
use crate::economy::{Building, Good, PlayerNation, Stockpile, Workforce};
use crate::ui::button_style::*;
use crate::ui::city::components::{
    AdjustProductionButton, ProductionChoiceButton, ProductionLaborDisplay, ProductionTargetDisplay,
};

use super::types::BuildingDialog;

/// Populate production dialog content (Rendering Layer)
/// Called when a production building dialog is opened
pub fn populate_production_dialog(
    mut commands: Commands,
    new_dialogs: Query<&BuildingDialog, Added<BuildingDialog>>,
    buildings: Query<(&Building, &ProductionSettings)>,
    player_nation: Option<Res<PlayerNation>>,
    stockpiles: Query<&Stockpile>,
    workforces: Query<&Workforce>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(stockpile) = stockpiles.get(player.0) else {
        return;
    };

    let Ok(workforce) = workforces.get(player.0) else {
        return;
    };

    for dialog in new_dialogs.iter() {
        // Only handle production buildings
        match dialog.building_kind {
            BuildingKind::TextileMill
            | BuildingKind::LumberMill
            | BuildingKind::SteelMill
            | BuildingKind::FoodProcessingCenter => {}
            _ => continue, // Not a production building
        }

        let Ok((building, settings)) = buildings.get(dialog.building_entity) else {
            continue;
        };

        let content_entity = dialog.content_entity;

        // Populate content based on building kind
        spawn_production_content(
            &mut commands,
            content_entity,
            dialog.building_entity,
            building,
            settings,
            stockpile,
            workforce,
        );
    }
}

/// Spawn production dialog content
fn spawn_production_content(
    commands: &mut Commands,
    content_entity: Entity,
    building_entity: Entity,
    building: &Building,
    settings: &ProductionSettings,
    stockpile: &Stockpile,
    workforce: &Workforce,
) {
    let building_kind = building.kind;
    let choice = settings.choice;
    let _target_output = settings.target_output;
    let _capacity = building.capacity;
    let _available_labor = workforce.available_labor();

    // Clone values needed for the closure
    let stockpile_clone = stockpile.clone();

    commands
        .entity(content_entity)
        .with_children(move |content| {
            // Production equation section - INLINED
            content
                .spawn(Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                })
                .with_children(|equation| {
                    // Get recipe based on building kind and choice
                    let (inputs, output) = get_recipe(building_kind, choice);

                    // Display inputs
                    for (i, (good, amount)) in inputs.iter().enumerate() {
                        if i > 0 {
                            equation.spawn((
                                Text::new("+"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                            ));
                        }

                        // Check if we have enough of this input (use available, not total)
                        let available = stockpile_clone.get_available(*good);
                        let has_enough = available >= *amount;

                        // Input icon/text
                        equation
                            .spawn(Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            })
                            .with_children(|icon| {
                                icon.spawn((
                                    Text::new(format!("{}×\n{:?}", amount, good)),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(if has_enough {
                                        Color::srgb(0.9, 0.9, 0.9)
                                    } else {
                                        Color::srgb(0.9, 0.5, 0.5)
                                    }),
                                    TextLayout {
                                        justify: Justify::Center,
                                        ..default()
                                    },
                                ));

                                // Red X overlay if missing
                                if !has_enough {
                                    icon.spawn((
                                        Text::new("✗"),
                                        TextFont {
                                            font_size: 48.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(1.0, 0.2, 0.2)),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            ..default()
                                        },
                                    ));
                                }
                            });
                    }

                    // Arrow
                    equation.spawn((
                        Text::new("→"),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));

                    // Output
                    let (out_good, out_amount) = output;
                    equation
                        .spawn(Node {
                            width: Val::Px(80.0),
                            height: Val::Px(80.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        })
                        .with_children(|icon| {
                            icon.spawn((
                                Text::new(format!("{}×\n{:?}", out_amount, out_good)),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.9, 0.7)),
                                TextLayout {
                                    justify: Justify::Center,
                                    ..default()
                                },
                            ));
                        });
                });

            // Choice buttons (if applicable) - INLINED
            let choices: Option<Vec<(&str, ProductionChoice)>> = match building_kind {
                BuildingKind::TextileMill => Some(vec![
                    ("Use Cotton", ProductionChoice::UseCotton),
                    ("Use Wool", ProductionChoice::UseWool),
                ]),
                BuildingKind::LumberMill => Some(vec![
                    ("Make Lumber", ProductionChoice::MakeLumber),
                    ("Make Paper", ProductionChoice::MakePaper),
                ]),
                BuildingKind::FoodProcessingCenter => Some(vec![
                    ("Use Livestock", ProductionChoice::UseLivestock),
                    ("Use Fish", ProductionChoice::UseFish),
                ]),
                _ => None,
            };

            if let Some(choices) = choices {
                content
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        for (label, choice_opt) in choices {
                            let is_selected = choice_opt == choice;
                            row.spawn((
                                Button,
                                Node {
                                    padding: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(if is_selected {
                                    Color::srgba(0.3, 0.4, 0.5, 1.0)
                                } else {
                                    NORMAL_BUTTON
                                }),
                                BorderColor::all(if is_selected {
                                    Color::srgba(0.5, 0.7, 0.9, 1.0)
                                } else {
                                    Color::srgba(0.5, 0.5, 0.6, 0.8)
                                }),
                                ProductionChoiceButton {
                                    building_entity,
                                    choice: choice_opt,
                                },
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                ));
                            });
                        }
                    });
            }

            // Capacity and output section
            content
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                })
                .with_children(|section| {
                    // Capacity display
                    section.spawn((
                        Text::new(format!("Capacity: {} units/turn", building.capacity)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));

                    // Output control row
                    section
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Target Output:"),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));

                            // +/- buttons
                            row.spawn(Node {
                                column_gap: Val::Px(8.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|controls| {
                                // Decrease button
                                controls
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(40.0),
                                            height: Val::Px(40.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(NORMAL_BUTTON),
                                        BorderColor::all(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                                        AdjustProductionButton {
                                            building_entity,
                                            delta: -1,
                                        },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new("−"),
                                            TextFont {
                                                font_size: 24.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                        ));
                                    });

                                // Current output display
                                controls.spawn((
                                    Text::new(format!("{}", settings.target_output)),
                                    TextFont {
                                        font_size: 20.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 1.0, 0.6)),
                                    ProductionTargetDisplay { building_entity },
                                ));

                                // Increase button
                                controls
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(40.0),
                                            height: Val::Px(40.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(NORMAL_BUTTON),
                                        BorderColor::all(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                                        AdjustProductionButton {
                                            building_entity,
                                            delta: 1,
                                        },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new("+"),
                                            TextFont {
                                                font_size: 24.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                        ));
                                    });
                            });
                        });
                });

            // Labor cost section
            let available_labor = workforce.available_labor();
            content.spawn((
                Text::new(format!(
                    "Labor: {} units required (available: {})",
                    settings.target_output, available_labor
                )),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(if settings.target_output <= available_labor {
                    Color::srgb(0.7, 0.9, 0.7)
                } else {
                    Color::srgb(0.9, 0.6, 0.6)
                }),
                ProductionLaborDisplay { building_entity },
            ));

            // TODO: Expand Industry button (Phase 5)
        });
}

/// Update production dialog target output display (Rendering Layer)
pub fn update_production_target_display(
    settings_query: Query<&ProductionSettings, Changed<ProductionSettings>>,
    mut display_query: Query<(&mut Text, &ProductionTargetDisplay)>,
) {
    for (mut text, display) in display_query.iter_mut() {
        if let Ok(settings) = settings_query.get(display.building_entity) {
            **text = format!("{}", settings.target_output);
        }
    }
}

/// Update production dialog labor display (Rendering Layer)
pub fn update_production_labor_display(
    player_nation: Option<Res<PlayerNation>>,
    settings_query: Query<&ProductionSettings>,
    workforce_query: Query<&Workforce>,
    mut display_query: Query<(&mut Text, &mut TextColor, &ProductionLaborDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.0) else {
        return;
    };

    let available_labor = workforce.available_labor();

    for (mut text, mut color, display) in display_query.iter_mut() {
        if let Ok(settings) = settings_query.get(display.building_entity) {
            **text = format!(
                "Labor: {} units required (available: {})",
                settings.target_output, available_labor
            );

            *color = TextColor(if settings.target_output <= available_labor {
                Color::srgb(0.7, 0.9, 0.7)
            } else {
                Color::srgb(0.9, 0.6, 0.6)
            });
        }
    }
}

/// Get recipe for a building and choice
fn get_recipe(
    building_kind: BuildingKind,
    choice: ProductionChoice,
) -> (Vec<(Good, u32)>, (Good, u32)) {
    match building_kind {
        BuildingKind::TextileMill => {
            let input = match choice {
                ProductionChoice::UseCotton => Good::Cotton,
                ProductionChoice::UseWool => Good::Wool,
                _ => Good::Cotton,
            };
            (vec![(input, 2)], (Good::Fabric, 1))
        }
        BuildingKind::LumberMill => {
            let output = match choice {
                ProductionChoice::MakeLumber => Good::Lumber,
                ProductionChoice::MakePaper => Good::Paper,
                _ => Good::Lumber,
            };
            (vec![(Good::Timber, 2)], (output, 1))
        }
        BuildingKind::SteelMill => (vec![(Good::Iron, 1), (Good::Coal, 1)], (Good::Steel, 1)),
        BuildingKind::FoodProcessingCenter => {
            let meat = match choice {
                ProductionChoice::UseLivestock => Good::Livestock,
                ProductionChoice::UseFish => Good::Fish,
                _ => Good::Livestock,
            };
            (
                vec![(Good::Grain, 2), (Good::Fruit, 1), (meat, 1)],
                (Good::CannedFood, 2),
            )
        }
        _ => (vec![], (Good::Fabric, 0)), // Shouldn't happen
    }
}
