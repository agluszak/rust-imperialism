use bevy::prelude::*;

use crate::economy::production::{Building, BuildingKind, Buildings, ProductionSettings};
use crate::economy::{Good, PlayerNation, Stockpile, Workforce};
use crate::ui::city::allocation_widgets::AllocationType;
use crate::ui::city::components::ProductionLaborDisplay;

use super::types::BuildingDialog;

/// Populate production dialog content (Rendering Layer)
/// Called when a production building dialog is opened
pub fn populate_production_dialog(
    mut commands: Commands,
    new_dialogs: Query<&BuildingDialog, Added<BuildingDialog>>,
    buildings_collections: Query<&Buildings>,
    settings_query: Query<&ProductionSettings>,
    player_nation: Option<Res<PlayerNation>>,
    stockpiles: Query<&Stockpile>,
    workforces: Query<&Workforce>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(stockpile) = stockpiles.get(player.entity()) else {
        return;
    };

    let Ok(workforce) = workforces.get(player.entity()) else {
        return;
    };

    let Ok(buildings_collection) = buildings_collections.get(player.entity()) else {
        return;
    };

    let Ok(settings) = settings_query.get(player.entity()) else {
        return;
    };

    for dialog in new_dialogs.iter() {
        // Only handle production buildings
        match dialog.building_kind {
            BuildingKind::TextileMill
            | BuildingKind::ClothingFactory
            | BuildingKind::LumberMill
            | BuildingKind::FurnitureFactory
            | BuildingKind::SteelMill
            | BuildingKind::MetalWorks
            | BuildingKind::FoodProcessingCenter
            | BuildingKind::Refinery
            | BuildingKind::Railyard => {}
            _ => continue, // Not a production building
        }

        let Some(building) = buildings_collection.get(dialog.building_kind) else {
            continue;
        };

        let content_entity = dialog.content_entity;

        // Populate content based on building kind
        spawn_production_content(
            &mut commands,
            content_entity,
            dialog.building_entity,
            &building,
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
    _settings: &ProductionSettings,
    stockpile: &Stockpile,
    workforce: &Workforce,
) {
    let building_kind = building.kind;
    let _capacity = building.capacity;
    let _available_labor = workforce.available_labor();

    // Clone values needed for the closure
    let stockpile_clone = stockpile.clone();

    // Determine which output goods this building can produce
    let output_goods = match building_kind {
        BuildingKind::TextileMill => vec![Good::Fabric],
        BuildingKind::ClothingFactory => vec![Good::Clothing],
        BuildingKind::LumberMill => vec![Good::Lumber, Good::Paper], // TWO separate outputs!
        BuildingKind::FurnitureFactory => vec![Good::Furniture],
        BuildingKind::SteelMill => vec![Good::Steel],
        BuildingKind::MetalWorks => vec![Good::Hardware, Good::Armaments],
        BuildingKind::FoodProcessingCenter => vec![Good::CannedFood],
        BuildingKind::Refinery => vec![Good::Fuel],
        BuildingKind::Railyard => vec![Good::Transport],
        _ => vec![],
    };

    // Building title and capacity
    commands.entity(content_entity).with_children(|content| {
        let capacity_text = if building.capacity == u32::MAX {
            "∞".to_string()
        } else {
            building.capacity.to_string()
        };
        content.spawn((
            Text::new(format!("{:?} (Cap: {})", building_kind, capacity_text)),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 1.0)),
            Node {
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
        ));
    });

    // For each output good, show a production section
    for output_good in output_goods.iter() {
        spawn_production_section(
            commands,
            content_entity,
            building_entity,
            building_kind,
            *output_good,
            &stockpile_clone,
            workforce,
        );
    }
}

/// Spawn a single production section (recipe + allocation UI) for one output
fn spawn_production_section(
    commands: &mut Commands,
    parent_entity: Entity,
    building_entity: Entity,
    building_kind: BuildingKind,
    output_good: Good,
    stockpile: &Stockpile,
    workforce: &Workforce,
) {
    commands.entity(parent_entity).with_children(|content| {
        content
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            })
            .with_children(|section| {
                // Section title
                section.spawn((
                    Text::new(format!("→ {:?}", output_good)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.9, 1.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                ));

                // Production equation section
                section
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    })
                    .with_children(|equation| {
                        // Get recipe for this specific output
                        let (input_alternatives, output) =
                            get_recipe_for_output(building_kind, output_good);

                        // Display input alternatives (e.g., "2× Cotton OR 2× Wool")
                        for (alt_idx, alternative) in input_alternatives.iter().enumerate() {
                            if alt_idx > 0 {
                                // Show "OR" between alternatives
                                equation.spawn((
                                    Text::new("OR"),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.9, 0.7, 0.5)),
                                    Node {
                                        margin: UiRect::horizontal(Val::Px(4.0)),
                                        ..default()
                                    },
                                ));
                            }

                            // Show this alternative's inputs
                            for (i, (good, amount)) in alternative.iter().enumerate() {
                                if i > 0 {
                                    // Show "+" between inputs in same alternative
                                    equation.spawn((
                                        Text::new("+"),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                    ));
                                }

                                // Check if we have enough of this input (use available, not total)
                                let available = stockpile.get_available(*good);
                                let has_enough = available >= *amount;

                                // Input icon/text
                                equation
                                    .spawn(Node {
                                        width: Val::Px(55.0),
                                        height: Val::Px(55.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    })
                                    .with_children(|icon| {
                                        icon.spawn((
                                            Text::new(format!("{}×\n{:?}", amount, good)),
                                            TextFont {
                                                font_size: 10.0,
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
                                                    font_size: 32.0,
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
                        }

                        // Arrow
                        equation.spawn((
                            Text::new("→"),
                            TextFont {
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        ));

                        // Output
                        let (out_good, out_amount) = output;
                        equation
                            .spawn(Node {
                                width: Val::Px(55.0),
                                height: Val::Px(55.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            })
                            .with_children(|icon| {
                                icon.spawn((
                                    Text::new(format!("{}×\n{:?}", out_amount, out_good)),
                                    TextFont {
                                        font_size: 10.0,
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

                // Allocation UI using widget macros
                let allocation_type = AllocationType::Production(building_entity, output_good);

                // Stepper for target output
                crate::spawn_allocation_stepper!(section, "Target Production", allocation_type);

                // Resource allocation bars - show ALL possible inputs
                let (input_alternatives, _output) =
                    get_recipe_for_output(building_kind, output_good);
                // Collect all unique goods from all alternatives
                let mut all_goods: Vec<Good> = Vec::new();
                for alternative in input_alternatives.iter() {
                    for (good, _amount) in alternative.iter() {
                        if !all_goods.contains(good) {
                            all_goods.push(*good);
                        }
                    }
                }
                // Show allocation bars for each unique good
                for good in all_goods.iter() {
                    let good_name = format!("{:?}", good);
                    crate::spawn_allocation_bar!(section, *good, &good_name, allocation_type);
                }

                // Labor allocation bar (showing labor as a resource)
                // Note: Labor will be dynamically updated by the update_production_labor_display system
                section
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    })
                    .with_children(|bar_container| {
                        bar_container.spawn((
                            Text::new("Labor"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));

                        bar_container.spawn((
                            Text::new(format!(
                                "Required: 0 (Available: {})",
                                workforce.available_labor()
                            )),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.9, 0.7)),
                            ProductionLaborDisplay {
                                building_entity,
                                output_good,
                            },
                        ));
                    });

                // Summary
                crate::spawn_allocation_summary!(section, allocation_type);
            });
    });
}

/// Update production dialog labor display (Rendering Layer)
/// This updates the custom labor display that isn't part of the standard allocation bars
pub fn update_production_labor_display(
    player_nation: Option<Res<PlayerNation>>,
    allocations_query: Query<&crate::economy::Allocations>,
    workforce_query: Query<&Workforce>,
    mut display_query: Query<(&mut Text, &mut TextColor, &ProductionLaborDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.entity()) else {
        return;
    };

    let Ok(allocations) = allocations_query.get(player.entity()) else {
        return;
    };

    let available_labor = workforce.available_labor();

    for (mut text, mut color, display) in display_query.iter_mut() {
        // Get production allocation for this specific output_good
        let production_alloc =
            allocations.production_count(display.building_entity, display.output_good) as u32;

        **text = format!(
            "Required: {} (Available: {})",
            production_alloc, available_labor
        );

        *color = TextColor(if production_alloc <= available_labor {
            Color::srgb(0.7, 0.9, 0.7)
        } else {
            Color::srgb(0.9, 0.6, 0.6)
        });
    }
}

/// Get recipe for a building and choice
/// Get recipe for a specific output good
/// Returns (inputs, output) where inputs shows ALL possible alternatives
/// For TextileMill: shows "2× Cotton OR 2× Wool"
/// For FoodProcessing: shows "2× Grain + 1× Fruit + 1× (Livestock OR Fish)"
fn get_recipe_for_output(
    building_kind: BuildingKind,
    output_good: Good,
) -> (Vec<Vec<(Good, u32)>>, (Good, u32)) {
    match (building_kind, output_good) {
        (BuildingKind::TextileMill, Good::Fabric) => {
            // Two alternatives: Cotton OR Wool
            (
                vec![vec![(Good::Cotton, 2)], vec![(Good::Wool, 2)]],
                (Good::Fabric, 1),
            )
        }
        (BuildingKind::ClothingFactory, Good::Clothing) => {
            // Simple: 2 Fabric → 1 Clothing
            (vec![vec![(Good::Fabric, 2)]], (Good::Clothing, 1))
        }
        (BuildingKind::LumberMill, Good::Lumber) => {
            // Simple: 2 Timber → 1 Lumber
            (vec![vec![(Good::Timber, 2)]], (Good::Lumber, 1))
        }
        (BuildingKind::LumberMill, Good::Paper) => {
            // Simple: 2 Timber → 1 Paper
            (vec![vec![(Good::Timber, 2)]], (Good::Paper, 1))
        }
        (BuildingKind::FurnitureFactory, Good::Furniture) => {
            // Simple: 2 Lumber → 1 Furniture
            (vec![vec![(Good::Lumber, 2)]], (Good::Furniture, 1))
        }
        (BuildingKind::SteelMill, Good::Steel) => {
            // Simple: 1 Iron + 1 Coal → 1 Steel
            (
                vec![vec![(Good::Iron, 1), (Good::Coal, 1)]],
                (Good::Steel, 1),
            )
        }
        (BuildingKind::MetalWorks, Good::Hardware) => {
            // Simple: 2 Steel → 1 Hardware
            (vec![vec![(Good::Steel, 2)]], (Good::Hardware, 1))
        }
        (BuildingKind::MetalWorks, Good::Armaments) => {
            // Simple: 2 Steel → 1 Armaments
            (vec![vec![(Good::Steel, 2)]], (Good::Armaments, 1))
        }
        (BuildingKind::FoodProcessingCenter, Good::CannedFood) => {
            // Complex: 2 Grain + 1 Fruit + (1 Livestock OR 1 Fish) → 2 CannedFood
            // Show as two alternatives: one with Livestock, one with Fish
            (
                vec![
                    vec![(Good::Grain, 2), (Good::Fruit, 1), (Good::Livestock, 1)],
                    vec![(Good::Grain, 2), (Good::Fruit, 1), (Good::Fish, 1)],
                ],
                (Good::CannedFood, 2),
            )
        }
        (BuildingKind::Refinery, Good::Fuel) => {
            // Simple: 2 Oil → 1 Fuel
            (vec![vec![(Good::Oil, 2)]], (Good::Fuel, 1))
        }
        (BuildingKind::Railyard, Good::Transport) => {
            // Simple: 1 Steel + 1 Lumber → 1 Transport
            (
                vec![vec![(Good::Steel, 1), (Good::Lumber, 1)]],
                (Good::Transport, 1),
            )
        }
        _ => (vec![], (Good::Fabric, 0)), // Shouldn't happen
    }
}
