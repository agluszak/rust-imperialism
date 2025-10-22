use bevy::prelude::*;

use crate::economy::production::BuildingKind;
use crate::economy::workforce::calculate_recruitment_cap;
use crate::economy::{
    Good, PlayerNation, RecruitmentCapacity, RecruitmentQueue, Stockpile, WorkerSkill, Workforce,
};
use crate::map::province::Province;
use crate::ui::city::allocation_widgets::AllocationType;
use crate::ui::city::components::{
    CapitolCapacityDisplay, CapitolRequirementDisplay, TradeSchoolPaperDisplay,
    TradeSchoolWorkforceDisplay,
};

use super::types::BuildingDialog;

/// Populate special building dialogs (Capitol, Trade School, Power Plant)
pub fn populate_special_dialog(
    mut commands: Commands,
    new_dialogs: Query<&BuildingDialog, Added<BuildingDialog>>,
    player_nation: Option<Res<PlayerNation>>,
    stockpiles: Query<&Stockpile>,
    workforces: Query<&Workforce>,
    recruitment_caps: Query<&RecruitmentCapacity>,
    recruitment_queues: Query<&RecruitmentQueue>,
    provinces: Query<&Province>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let player_entity = player.entity();

    let Ok(stockpile) = stockpiles.get(player_entity) else {
        return;
    };

    let Ok(workforce) = workforces.get(player_entity) else {
        return;
    };

    for dialog in new_dialogs.iter() {
        let content_entity = dialog.content_entity;

        match dialog.building_kind {
            BuildingKind::Capitol => {
                // Calculate province count
                let province_count = provinces
                    .iter()
                    .filter(|p| p.owner == Some(player_entity))
                    .count() as u32;

                spawn_capitol_content(
                    &mut commands,
                    content_entity,
                    stockpile,
                    province_count,
                    recruitment_caps.get(player_entity).ok(),
                    recruitment_queues.get(player_entity).ok(),
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
    recruitment_cap: Option<&RecruitmentCapacity>,
    recruitment_queue: Option<&RecruitmentQueue>,
) {
    let upgraded = recruitment_cap.map(|c| c.upgraded).unwrap_or(false);
    let cap = calculate_recruitment_cap(province_count, upgraded);
    let queued = recruitment_queue.map(|q| q.queued).unwrap_or(0);
    let _remaining = cap.saturating_sub(queued);

    // Check requirements: canned food, clothing, furniture
    let has_food = stockpile.get_available(Good::CannedFood) >= 1;
    let has_clothing = stockpile.get_available(Good::Clothing) >= 1;
    let has_furniture = stockpile.get_available(Good::Furniture) >= 1;

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
                        "  • 1x Canned Food {}",
                        if has_food { "[x]" } else { "[ ]" }
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
                        "  • 1x Clothing {}",
                        if has_clothing { "[x]" } else { "[ ]" }
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
                        "  • 1x Furniture {}",
                        if has_furniture { "[x]" } else { "[ ]" }
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
                "Capacity: {} per turn (based on {} provinces{})",
                cap,
                province_count,
                if upgraded { ", upgraded" } else { "" }
            )),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            CapitolCapacityDisplay,
        ));

        // NEW: Allocation stepper (using macro)
        crate::spawn_allocation_stepper!(content, "Allocate Workers", AllocationType::Recruitment);

        // Resource allocation section header
        content.spawn((
            Text::new("Resource Allocation:"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Node {
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
        ));

        // Allocation bars (using macro)
        for (good, name) in [
            (Good::CannedFood, "Canned Food"),
            (Good::Clothing, "Clothing"),
            (Good::Furniture, "Furniture"),
        ] {
            crate::spawn_allocation_bar!(content, good, name, AllocationType::Recruitment);
        }

        // Summary (using macro)
        crate::spawn_allocation_summary!(content, AllocationType::Recruitment);
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

        // Training options (NEW: using allocation macros)

        // Section 1: Train Untrained -> Trained
        content.spawn((
            Text::new("Train Untrained -> Trained"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
            Node {
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
        ));

        crate::spawn_allocation_stepper!(
            content,
            "Allocate Workers",
            AllocationType::Training(WorkerSkill::Untrained)
        );

        crate::spawn_allocation_bar!(
            content,
            Good::Paper,
            "Paper",
            AllocationType::Training(WorkerSkill::Untrained)
        );

        crate::spawn_allocation_summary!(content, AllocationType::Training(WorkerSkill::Untrained));

        // Section 2: Train Trained -> Expert
        content.spawn((
            Text::new("Train Trained -> Expert"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
            Node {
                margin: UiRect::top(Val::Px(24.0)),
                ..default()
            },
        ));

        crate::spawn_allocation_stepper!(
            content,
            "Allocate Workers",
            AllocationType::Training(WorkerSkill::Trained)
        );

        crate::spawn_allocation_bar!(
            content,
            Good::Paper,
            "Paper",
            AllocationType::Training(WorkerSkill::Trained)
        );

        crate::spawn_allocation_summary!(content, AllocationType::Training(WorkerSkill::Trained));
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
                    Text::new("Conversion: 1 fuel -> 2 labor points"),
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

    let player_entity = player.entity();

    let Ok(stockpile) = stockpile_query.get(player_entity) else {
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

        **text = format!(
            "  • 1x {} {}",
            good_name,
            if available { "[x]" } else { "[ ]" }
        );
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
    recruitment_cap_query: Query<&RecruitmentCapacity>,
    recruitment_queue_query: Query<&RecruitmentQueue, Changed<RecruitmentQueue>>,
    provinces: Query<&Province>,
    mut display_query: Query<(&mut Text, &mut TextColor), With<CapitolCapacityDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let player_entity = player.entity();

    let Ok(_queue) = recruitment_queue_query.get(player_entity) else {
        return;
    };

    let province_count = provinces
        .iter()
        .filter(|p| p.owner == Some(player_entity))
        .count() as u32;

    let upgraded = recruitment_cap_query
        .get(player_entity)
        .map(|c| c.upgraded)
        .unwrap_or(false);
    let cap = calculate_recruitment_cap(province_count, upgraded);
    let queued = recruitment_queue_query
        .get(player_entity)
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

    let player_entity = player.entity();

    let Ok(workforce) = workforce_query.get(player_entity) else {
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

    let player_entity = player.entity();

    let Ok(stockpile) = stockpile_query.get(player_entity) else {
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
