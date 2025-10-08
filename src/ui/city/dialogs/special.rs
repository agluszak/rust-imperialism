use bevy::prelude::*;

use crate::economy::production::BuildingKind;
use crate::economy::workforce::calculate_recruitment_cap;
use crate::economy::{Good, PlayerNation, Stockpile, Workforce};
use crate::ui::button_style::*;
use crate::ui::city::components::{
    CapitolCapacityDisplay, CapitolRequirementDisplay, RecruitWorkersButton,
    TradeSchoolPaperDisplay, TradeSchoolWorkforceDisplay, TrainWorkerButton,
};

use super::types::BuildingDialog;

/// Populate special building dialogs (Capitol, Trade School, Power Plant)
pub fn populate_special_dialog(
    mut commands: Commands,
    new_dialogs: Query<&BuildingDialog, Added<BuildingDialog>>,
    player_nation: Option<Res<PlayerNation>>,
    stockpiles: Query<&Stockpile>,
    workforces: Query<&Workforce>,
    recruitment_caps: Query<&crate::economy::RecruitmentCapacity>,
    recruitment_queues: Query<&crate::economy::RecruitmentQueue>,
    provinces: Query<&crate::province::Province>,
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
        let content_entity = dialog.content_entity;

        match dialog.building_kind {
            BuildingKind::Capitol => {
                // Calculate province count
                let province_count = provinces
                    .iter()
                    .filter(|p| p.owner == Some(player.0))
                    .count() as u32;

                spawn_capitol_content(
                    &mut commands,
                    content_entity,
                    stockpile,
                    province_count,
                    recruitment_caps.get(player.0).ok(),
                    recruitment_queues.get(player.0).ok(),
                );
            }
            BuildingKind::TradeSchool => {
                spawn_trade_school_content(&mut commands, content_entity, stockpile, workforce);
            }
            BuildingKind::PowerPlant => {
                // TODO: Power Plant needs different handling - fuel conversion
                spawn_power_plant_content(&mut commands, content_entity, stockpile);
            }
            _ => continue, // Not a special building
        }
    }
}

/// Spawn Capitol dialog content (worker recruitment)
fn spawn_capitol_content(
    commands: &mut Commands,
    content_entity: Entity,
    stockpile: &Stockpile,
    province_count: u32,
    recruitment_cap: Option<&crate::economy::RecruitmentCapacity>,
    recruitment_queue: Option<&crate::economy::RecruitmentQueue>,
) {
    let upgraded = recruitment_cap.map(|c| c.upgraded).unwrap_or(false);
    let cap = calculate_recruitment_cap(province_count, upgraded);
    let queued = recruitment_queue.map(|q| q.queued).unwrap_or(0);
    let remaining = cap.saturating_sub(queued);

    // Check requirements: canned food, clothing, furniture
    let has_food = stockpile.get_available(Good::CannedFood) >= 1;
    let has_clothing = stockpile.get_available(Good::Clothing) >= 1;
    let has_furniture = stockpile.get_available(Good::Furniture) >= 1;
    let can_recruit = has_food && has_clothing && has_furniture && remaining > 0;

    commands.entity(content_entity).with_children(|content| {
        // Title
        content.spawn((
            Text::new("Worker Recruitment"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
        ));

        // Requirements section
        content
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            })
            .with_children(|section| {
                section.spawn((
                    Text::new("Requirements per worker:"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));

                // Canned Food
                section.spawn((
                    Text::new(format!(
                        "  • 1× Canned Food {}",
                        if has_food { "✓" } else { "✗" }
                    )),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(if has_food {
                        Color::srgb(0.7, 0.9, 0.7)
                    } else {
                        Color::srgb(0.9, 0.6, 0.6)
                    }),
                    CapitolRequirementDisplay {
                        good: Good::CannedFood,
                    },
                ));

                // Clothing
                section.spawn((
                    Text::new(format!(
                        "  • 1× Clothing {}",
                        if has_clothing { "✓" } else { "✗" }
                    )),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(if has_clothing {
                        Color::srgb(0.7, 0.9, 0.7)
                    } else {
                        Color::srgb(0.9, 0.6, 0.6)
                    }),
                    CapitolRequirementDisplay {
                        good: Good::Clothing,
                    },
                ));

                // Furniture
                section.spawn((
                    Text::new(format!(
                        "  • 1× Furniture {}",
                        if has_furniture { "✓" } else { "✗" }
                    )),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(if has_furniture {
                        Color::srgb(0.7, 0.9, 0.7)
                    } else {
                        Color::srgb(0.9, 0.6, 0.6)
                    }),
                    CapitolRequirementDisplay {
                        good: Good::Furniture,
                    },
                ));
            });

        // Capacity display
        content.spawn((
            Text::new(format!(
                "Recruitment capacity: {} / {} used this turn",
                queued, cap
            )),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(if remaining > 0 {
                Color::srgb(0.9, 0.9, 0.9)
            } else {
                Color::srgb(0.9, 0.6, 0.6)
            }),
            CapitolCapacityDisplay,
        ));

        // Recruit buttons
        content
            .spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            })
            .with_children(|row| {
                for count in [1, 5, 10] {
                    let enabled = can_recruit && count <= remaining;
                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(12.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(if enabled {
                            NORMAL_BUTTON
                        } else {
                            Color::srgba(0.2, 0.2, 0.2, 1.0)
                        }),
                        BorderColor::all(if enabled {
                            Color::srgba(0.5, 0.5, 0.6, 0.8)
                        } else {
                            Color::srgba(0.3, 0.3, 0.3, 0.8)
                        }),
                        RecruitWorkersButton { count },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("Recruit {}", count)),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(if enabled {
                                Color::srgb(0.9, 0.9, 1.0)
                            } else {
                                Color::srgb(0.5, 0.5, 0.5)
                            }),
                        ));
                    });
                }
            });
    });
}

/// Spawn Trade School dialog content (worker training)
fn spawn_trade_school_content(
    commands: &mut Commands,
    content_entity: Entity,
    stockpile: &Stockpile,
    workforce: &Workforce,
) {
    let untrained_count = workforce.untrained_count();
    let trained_count = workforce.trained_count();
    let expert_count = workforce.expert_count();

    // Check if we have paper for training
    let paper_available = stockpile.get_available(Good::Paper);

    commands.entity(content_entity).with_children(|content| {
        // Title
        content.spawn((
            Text::new("Worker Training"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
        ));

        // Current workforce display
        content
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                TradeSchoolWorkforceDisplay,
            ))
            .with_children(|section| {
                section.spawn((
                    Text::new("Current Workforce:"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));

                section.spawn((
                    Text::new(format!("  • Untrained: {} (1 labor each)", untrained_count)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));

                section.spawn((
                    Text::new(format!("  • Trained: {} (2 labor each)", trained_count)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));

                section.spawn((
                    Text::new(format!("  • Expert: {} (4 labor each)", expert_count)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            });

        // Paper requirement
        content.spawn((
            Text::new(format!(
                "Paper available: {} (1 required per training)",
                paper_available
            )),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(if paper_available > 0 {
                Color::srgb(0.7, 0.9, 0.7)
            } else {
                Color::srgb(0.9, 0.6, 0.6)
            }),
            TradeSchoolPaperDisplay,
        ));

        // Training options
        content
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            })
            .with_children(|section| {
                // Untrained → Trained
                section
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("Untrained → Trained (1 paper)"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));

                        let can_train = paper_available > 0 && untrained_count > 0;
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(if can_train {
                                NORMAL_BUTTON
                            } else {
                                Color::srgba(0.2, 0.2, 0.2, 1.0)
                            }),
                            BorderColor::all(if can_train {
                                Color::srgba(0.5, 0.5, 0.6, 0.8)
                            } else {
                                Color::srgba(0.3, 0.3, 0.3, 0.8)
                            }),
                            TrainWorkerButton {
                                from_skill: crate::economy::WorkerSkill::Untrained,
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Train"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(if can_train {
                                    Color::srgb(0.9, 0.9, 1.0)
                                } else {
                                    Color::srgb(0.5, 0.5, 0.5)
                                }),
                            ));
                        });
                    });

                // Trained → Expert
                section
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("Trained → Expert (1 paper)"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));

                        let can_train = paper_available > 0 && trained_count > 0;
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(if can_train {
                                NORMAL_BUTTON
                            } else {
                                Color::srgba(0.2, 0.2, 0.2, 1.0)
                            }),
                            BorderColor::all(if can_train {
                                Color::srgba(0.5, 0.5, 0.6, 0.8)
                            } else {
                                Color::srgba(0.3, 0.3, 0.3, 0.8)
                            }),
                            TrainWorkerButton {
                                from_skill: crate::economy::WorkerSkill::Trained,
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Train"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(if can_train {
                                    Color::srgb(0.9, 0.9, 1.0)
                                } else {
                                    Color::srgb(0.5, 0.5, 0.5)
                                }),
                            ));
                        });
                    });
            });
    });
}

/// Spawn Power Plant dialog content (fuel conversion)
fn spawn_power_plant_content(
    commands: &mut Commands,
    content_entity: Entity,
    stockpile: &Stockpile,
) {
    let fuel_available = stockpile.get_available(Good::Fuel);

    commands.entity(content_entity).with_children(|content| {
        // Title
        content.spawn((
            Text::new("Power Plant"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
        ));

        // Info section
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
                section.spawn((
                    Text::new(
                        "The Power Plant converts fuel into additional labor points each turn.",
                    ),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));

                section.spawn((
                    Text::new(format!("Fuel available: {}", fuel_available)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(if fuel_available > 0 {
                        Color::srgb(0.7, 0.9, 0.7)
                    } else {
                        Color::srgb(0.9, 0.6, 0.6)
                    }),
                ));

                section.spawn((
                    Text::new("Conversion: 1 fuel → 2 labor points"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));

                section.spawn((
                    Text::new("Note: Conversion happens automatically during production phase."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                ));
            });

        // TODO: Add slider/buttons to control fuel consumption if needed
    });
}

/// Update Capitol requirement displays when stockpile changes
pub fn update_capitol_requirement_displays(
    player_nation: Option<Res<PlayerNation>>,
    stockpile_query: Query<&Stockpile, Changed<Stockpile>>,
    mut display_query: Query<(&mut Text, &mut TextColor, &CapitolRequirementDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(stockpile) = stockpile_query.get(player.0) else {
        return;
    };

    for (mut text, mut color, display) in display_query.iter_mut() {
        let available = stockpile.get_available(display.good) >= 1;
        let good_name = match display.good {
            Good::CannedFood => "Canned Food",
            Good::Clothing => "Clothing",
            Good::Furniture => "Furniture",
            _ => continue,
        };

        **text = format!("  • 1× {} {}", good_name, if available { "✓" } else { "✗" });
        *color = TextColor(if available {
            Color::srgb(0.7, 0.9, 0.7)
        } else {
            Color::srgb(0.9, 0.6, 0.6)
        });
    }
}

/// Update Capitol capacity display when recruitment queue changes
pub fn update_capitol_capacity_display(
    player_nation: Option<Res<PlayerNation>>,
    recruitment_cap_query: Query<&crate::economy::RecruitmentCapacity>,
    recruitment_queue_query: Query<
        &crate::economy::RecruitmentQueue,
        Changed<crate::economy::RecruitmentQueue>,
    >,
    provinces: Query<&crate::province::Province>,
    mut display_query: Query<(&mut Text, &mut TextColor), With<CapitolCapacityDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(_queue) = recruitment_queue_query.get(player.0) else {
        return;
    };

    let province_count = provinces
        .iter()
        .filter(|p| p.owner == Some(player.0))
        .count() as u32;

    let upgraded = recruitment_cap_query
        .get(player.0)
        .map(|c| c.upgraded)
        .unwrap_or(false);
    let cap = calculate_recruitment_cap(province_count, upgraded);
    let queued = recruitment_queue_query
        .get(player.0)
        .map(|q| q.queued)
        .unwrap_or(0);
    let remaining = cap.saturating_sub(queued);

    for (mut text, mut color) in display_query.iter_mut() {
        **text = format!("Recruitment capacity: {} / {} used this turn", queued, cap);
        *color = TextColor(if remaining > 0 {
            Color::srgb(0.9, 0.9, 0.9)
        } else {
            Color::srgb(0.9, 0.6, 0.6)
        });
    }
}

/// Update Trade School workforce display when workforce changes
pub fn update_trade_school_workforce_display(
    player_nation: Option<Res<PlayerNation>>,
    workforce_query: Query<&Workforce, Changed<Workforce>>,
    display_query: Query<Entity, With<TradeSchoolWorkforceDisplay>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.0) else {
        return;
    };

    let untrained_count = workforce.untrained_count();
    let trained_count = workforce.trained_count();
    let expert_count = workforce.expert_count();

    // Find the workforce display container and update its children
    for container in display_query.iter() {
        if let Ok(children) = children_query.get(container) {
            // Skip the first child (title "Current Workforce:")
            // Then update the three count displays
            for (idx, child) in children.iter().skip(1).enumerate() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    match idx {
                        0 => **text = format!("  • Untrained: {} (1 labor each)", untrained_count),
                        1 => **text = format!("  • Trained: {} (2 labor each)", trained_count),
                        2 => **text = format!("  • Expert: {} (4 labor each)", expert_count),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Update Trade School paper display when stockpile changes
pub fn update_trade_school_paper_display(
    player_nation: Option<Res<PlayerNation>>,
    stockpile_query: Query<&Stockpile, Changed<Stockpile>>,
    mut display_query: Query<(&mut Text, &mut TextColor), With<TradeSchoolPaperDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(stockpile) = stockpile_query.get(player.0) else {
        return;
    };

    let paper_available = stockpile.get_available(Good::Paper);

    for (mut text, mut color) in display_query.iter_mut() {
        **text = format!(
            "Paper available: {} (1 required per training)",
            paper_available
        );
        *color = TextColor(if paper_available > 0 {
            Color::srgb(0.7, 0.9, 0.7)
        } else {
            Color::srgb(0.9, 0.6, 0.6)
        });
    }
}
