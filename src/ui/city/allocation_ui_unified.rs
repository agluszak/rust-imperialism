use bevy::prelude::*;

use super::allocation_widgets::{
    AllocationBar, AllocationStepperButton, AllocationStepperDisplay, AllocationSummary,
    AllocationType,
};
use crate::economy::{
    AdjustProduction, AdjustRecruitment, AdjustTraining, PlayerNation, ResourceAllocations,
    Stockpile,
};

// ============================================================================
// Input Layer: Unified stepper button handler
// ============================================================================

/// Handle ALL stepper button clicks (recruitment, training, production)
pub fn handle_all_stepper_buttons(
    interactions: Query<(&Interaction, &AllocationStepperButton), Changed<Interaction>>,
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations>,
    mut recruit_writer: MessageWriter<AdjustRecruitment>,
    mut train_writer: MessageWriter<AdjustTraining>,
    mut prod_writer: MessageWriter<AdjustProduction>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(alloc) = allocations.get(player.0) else {
        return;
    };

    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            match button.allocation_type {
                AllocationType::Recruitment => {
                    let current = alloc.recruitment.requested;
                    let new_requested = (current as i32 + button.delta).max(0) as u32;
                    recruit_writer.write(AdjustRecruitment {
                        nation: player.0,
                        requested: new_requested,
                    });
                    info!(
                        "Recruitment: {} → {} (delta: {})",
                        current, new_requested, button.delta
                    );
                }

                AllocationType::Training(from_skill) => {
                    let current = alloc
                        .training
                        .iter()
                        .find(|t| t.from_skill == from_skill)
                        .map(|t| t.requested)
                        .unwrap_or(0);
                    let new_requested = (current as i32 + button.delta).max(0) as u32;
                    train_writer.write(AdjustTraining {
                        nation: player.0,
                        from_skill,
                        requested: new_requested,
                    });
                    info!(
                        "Training ({:?}): {} → {} (delta: {})",
                        from_skill, current, new_requested, button.delta
                    );
                }

                AllocationType::Production(building_entity, output_good) => {
                    let current = alloc
                        .production
                        .get(&building_entity)
                        .and_then(|p| p.outputs.get(&output_good))
                        .map(|o| o.requested)
                        .unwrap_or(0);
                    let new_target = (current as i32 + button.delta).max(0) as u32;
                    prod_writer.write(AdjustProduction {
                        nation: player.0,
                        building: building_entity,
                        output_good,
                        choice: None, // Keep current choice
                        target_output: new_target,
                    });
                    info!(
                        "Production ({:?}): {} → {} (delta: {})",
                        output_good, current, new_target, button.delta
                    );
                }
            }
        }
    }
}

// ============================================================================
// Rendering Layer: Unified display updates
// ============================================================================

/// Update ALL stepper displays (recruitment, training, production)
pub fn update_all_stepper_displays(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    mut displays: Query<(&mut Text, &AllocationStepperDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        for (mut text, display) in displays.iter_mut() {
            let (requested, allocated) = match display.allocation_type {
                AllocationType::Recruitment => {
                    (alloc.recruitment.requested, alloc.recruitment.allocated)
                }

                AllocationType::Training(from_skill) => {
                    if let Some(training) =
                        alloc.training.iter().find(|t| t.from_skill == from_skill)
                    {
                        (training.requested, training.allocated)
                    } else {
                        (0, 0)
                    }
                }

                AllocationType::Production(building_entity, output_good) => {
                    if let Some(prod) = alloc.production.get(&building_entity) {
                        if let Some(output_alloc) = prod.outputs.get(&output_good) {
                            (output_alloc.requested, output_alloc.allocated)
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                }
            };

            // Show both requested and allocated if they differ
            text.0 = if requested == allocated {
                format!("{}", allocated)
            } else {
                format!("{} (want: {})", allocated, requested)
            };
        }
    }
}

/// Update ALL allocation bars (recruitment, training, production)
pub fn update_all_allocation_bars(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    stockpiles: Query<&Stockpile>,
    mut bars: Query<(
        &mut Text,
        &mut BackgroundColor,
        &mut BorderColor,
        &AllocationBar,
    )>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(alloc) = allocations.get(player.0) else {
        return;
    };

    let Ok(stockpile) = stockpiles.get(player.0) else {
        return;
    };

    for (mut text, mut bg_color, mut border_color, bar) in bars.iter_mut() {
        let available = stockpile.get_available(bar.good);

        // Calculate needed based on allocation type
        let needed = match bar.allocation_type {
            AllocationType::Recruitment => {
                // 1:1 ratio for recruitment goods
                alloc.recruitment.allocated
            }

            AllocationType::Training(_from_skill) => {
                // Find matching training allocation

                // 1:1 ratio for paper
                alloc
                    .training
                    .iter()
                    .find(|t| matches!(bar.allocation_type, AllocationType::Training(skill) if skill == t.from_skill))
                    .map(|t| t.allocated)
                    .unwrap_or(0)
            }

            AllocationType::Production(building_entity, _output_good) => {
                // Get production allocation and calculate inputs needed for ALL outputs
                if let Some(prod) = alloc.production.get(&building_entity) {
                    let inputs = prod.inputs_needed();
                    inputs
                        .iter()
                        .find(|(good, _)| *good == bar.good)
                        .map(|(_, qty)| *qty)
                        .unwrap_or(0)
                } else {
                    0
                }
            }
        };

        // Update text
        let good_name = format!("{:?}", bar.good); // Simple debug format for now
        text.0 = format!("{}: {} / {}", good_name, needed, available);

        // Color based on constraints
        let (bar_color, border_col) = if needed == 0 {
            // No allocation
            (
                Color::srgba(0.3, 0.3, 0.3, 0.8),
                Color::srgba(0.4, 0.4, 0.4, 0.8),
            )
        } else if needed <= available {
            // Can satisfy
            (
                Color::srgba(0.3, 0.7, 0.3, 0.9),
                Color::srgba(0.4, 0.8, 0.4, 1.0),
            )
        } else {
            // Insufficient
            (
                Color::srgba(0.8, 0.3, 0.3, 0.9),
                Color::srgba(0.9, 0.4, 0.4, 1.0),
            )
        };

        *bg_color = BackgroundColor(bar_color);
        *border_color = BorderColor::all(border_col);
    }
}

/// Update ALL allocation summaries
pub fn update_all_allocation_summaries(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    mut summaries: Query<(&mut Text, &AllocationSummary)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        for (mut text, summary) in summaries.iter_mut() {
            text.0 = match summary.allocation_type {
                AllocationType::Recruitment => {
                    let allocated = alloc.recruitment.allocated;
                    if allocated > 0 {
                        format!(
                            "→ Will recruit {} worker{} next turn",
                            allocated,
                            if allocated == 1 { "" } else { "s" }
                        )
                    } else {
                        "→ No workers will be recruited".to_string()
                    }
                }

                AllocationType::Training(from_skill) => {
                    let allocated = alloc
                        .training
                        .iter()
                        .find(|t| t.from_skill == from_skill)
                        .map(|t| t.allocated)
                        .unwrap_or(0);
                    if allocated > 0 {
                        let to_skill = from_skill.next_level();
                        format!(
                            "→ Will train {} worker{} from {:?} to {:?} next turn",
                            allocated,
                            if allocated == 1 { "" } else { "s" },
                            from_skill,
                            to_skill
                        )
                    } else {
                        "→ No workers will be trained".to_string()
                    }
                }

                AllocationType::Production(building_entity, output_good) => {
                    if let Some(prod) = alloc.production.get(&building_entity) {
                        if let Some(output_alloc) = prod.outputs.get(&output_good) {
                            let allocated = output_alloc.allocated;
                            if allocated > 0 {
                                format!("→ Will produce {} {:?} next turn", allocated, output_good)
                            } else {
                                "→ No production planned".to_string()
                            }
                        } else {
                            "→ No production planned".to_string()
                        }
                    } else {
                        "→ No production planned".to_string()
                    }
                }
            };
        }
    }
}
