use bevy::prelude::*;

use super::components::*;
use crate::economy::{
    AdjustRecruitment, AdjustTraining, PlayerNation, ResourceAllocations, Stockpile,
};

// ============================================================================
// Input Layer: Handle adjustment button clicks
// ============================================================================

/// Handle recruitment adjustment button clicks
pub fn handle_recruitment_adjustment_buttons(
    interactions: Query<(&Interaction, &AdjustRecruitmentButton), Changed<Interaction>>,
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations>,
    mut adjust_writer: MessageWriter<AdjustRecruitment>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(alloc) = allocations.get(player.0) else {
        return;
    };

    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            let current = alloc.recruitment.requested;
            let new_requested = (current as i32 + button.delta).max(0) as u32;

            info!(
                "Recruitment adjustment: {} → {} (delta: {})",
                current, new_requested, button.delta
            );

            adjust_writer.write(AdjustRecruitment {
                nation: player.0,
                requested: new_requested,
            });
        }
    }
}

/// Handle training adjustment button clicks
pub fn handle_training_adjustment_buttons(
    interactions: Query<(&Interaction, &AdjustTrainingButton), Changed<Interaction>>,
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations>,
    mut adjust_writer: MessageWriter<AdjustTraining>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(alloc) = allocations.get(player.0) else {
        return;
    };

    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find existing training allocation for this skill
            let current = alloc
                .training
                .iter()
                .find(|t| t.from_skill == button.from_skill)
                .map(|t| t.requested)
                .unwrap_or(0);

            let new_requested = (current as i32 + button.delta).max(0) as u32;

            info!(
                "Training adjustment ({:?}): {} → {} (delta: {})",
                button.from_skill, current, new_requested, button.delta
            );

            adjust_writer.write(AdjustTraining {
                nation: player.0,
                from_skill: button.from_skill,
                requested: new_requested,
            });
        }
    }
}

// ============================================================================
// Rendering Layer: Update allocation displays
// ============================================================================

/// Update recruitment allocation display (the number between +/- buttons)
pub fn update_recruitment_allocation_display(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    mut displays: Query<&mut Text, With<RecruitmentAllocationDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        let requested = alloc.recruitment.requested;
        let allocated = alloc.recruitment.allocated;

        for mut text in displays.iter_mut() {
            // Show both requested and allocated
            if requested == allocated {
                text.0 = format!("{}", allocated);
            } else {
                // Requested more than can be allocated
                text.0 = format!("{} (want: {})", allocated, requested);
            }
        }
    }
}

/// Update recruitment allocation bars (per-good resource bars)
pub fn update_recruitment_allocation_bars(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    stockpiles: Query<&Stockpile>,
    mut bars: Query<(
        &mut BackgroundColor,
        &mut BorderColor,
        &RecruitmentAllocationBar,
    )>,
    // Note: bar fill sizing would require restructuring the bar hierarchy
    // For now, we just update colors
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

    let allocated = alloc.recruitment.allocated;

    for (mut bg_color, mut border_color, bar) in bars.iter_mut() {
        let available = stockpile.get_available(bar.good);
        let needed = allocated; // 1:1 ratio for recruitment

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

        // Update fill bar width (assumes first child is the fill bar)
        // This requires the bar to have a child Node that represents the fill
        // For now, we'll just update the background color; proper fill bar needs restructuring
    }
}

/// Update training allocation displays
pub fn update_training_allocation_displays(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations, Changed<ResourceAllocations>>,
    mut displays: Query<(&mut Text, &TrainingAllocationDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        for (mut text, display) in displays.iter_mut() {
            // Find matching training allocation
            if let Some(training) = alloc
                .training
                .iter()
                .find(|t| t.from_skill == display.from_skill)
            {
                let requested = training.requested;
                let allocated = training.allocated;

                if requested == allocated {
                    text.0 = format!("{}", allocated);
                } else {
                    text.0 = format!("{} (want: {})", allocated, requested);
                }
            } else {
                text.0 = "0".to_string();
            }
        }
    }
}

// Note: Spawn functions are defined in dialogs/special.rs
// where they have access to the proper scope
