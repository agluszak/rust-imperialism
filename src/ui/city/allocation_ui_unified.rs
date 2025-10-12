use bevy::prelude::*;

use super::allocation_widgets::{
    AllocationBar, AllocationStepperButton, AllocationStepperDisplay, AllocationSummary,
    AllocationType,
};
use crate::economy::{
    AdjustProduction, AdjustRecruitment, AdjustTraining, Allocations, PlayerNation, Stockpile,
};

// ============================================================================
// Input Layer: Unified stepper button handler
// ============================================================================

/// Handle ALL stepper button clicks (recruitment, training, production)
pub fn handle_all_stepper_buttons(
    interactions: Query<(&Interaction, &AllocationStepperButton), Changed<Interaction>>,
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
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
                    let current = alloc.recruitment_count() as u32;
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
                    let current = alloc.training_count(from_skill) as u32;
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
                    let current = alloc.production_count(building_entity, output_good) as u32;
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
    allocations: Query<&Allocations, Changed<Allocations>>,
    mut displays: Query<(&mut Text, &AllocationStepperDisplay)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        for (mut text, display) in displays.iter_mut() {
            let allocated = match display.allocation_type {
                AllocationType::Recruitment => alloc.recruitment_count(),

                AllocationType::Training(from_skill) => alloc.training_count(from_skill),

                AllocationType::Production(building_entity, output_good) => {
                    alloc.production_count(building_entity, output_good)
                }
            };

            // With new system, allocated is always what's been successfully reserved
            text.0 = format!("{}", allocated);
        }
    }
}

/// Update ALL allocation bars (recruitment, training, production)
pub fn update_all_allocation_bars(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations, Changed<Allocations>>,
    stockpiles: Query<&Stockpile>,
    buildings: Query<&crate::economy::Building>,
    mut bars: Query<(
        &mut Text,
        &mut BackgroundColor,
        &mut BorderColor,
        &AllocationBar,
    )>,
) {
    use crate::economy::{BuildingKind, Good};

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
        // Each allocation represents 1 unit, so needed = count × per-unit-cost
        let needed = match bar.allocation_type {
            AllocationType::Recruitment => {
                // 1 CannedFood, 1 Clothing, 1 Furniture per worker
                let count = alloc.recruitment_count() as u32;
                match bar.good {
                    Good::CannedFood | Good::Clothing | Good::Furniture => count,
                    _ => 0,
                }
            }

            AllocationType::Training(from_skill) => {
                // 1 Paper per training
                let count = alloc.training_count(from_skill) as u32;
                match bar.good {
                    Good::Paper => count,
                    _ => 0,
                }
            }

            AllocationType::Production(building_entity, output_good) => {
                // Get count and building kind to calculate inputs
                let count = alloc.production_count(building_entity, output_good) as u32;
                if count == 0 {
                    0
                } else if let Ok(building) = buildings.get(building_entity) {
                    // Calculate per-unit cost and multiply by count
                    let per_unit_cost = match (building.kind, bar.good) {
                        (BuildingKind::TextileMill, Good::Cotton) => 2, // 2 cotton per fabric
                        (BuildingKind::TextileMill, Good::Wool) => 2,   // 2 wool per fabric
                        (BuildingKind::LumberMill, Good::Timber) => 2,  // 2 timber per output
                        (BuildingKind::SteelMill, Good::Iron) => 1,     // 1 iron per steel
                        (BuildingKind::SteelMill, Good::Coal) => 1,     // 1 coal per steel
                        (BuildingKind::FoodProcessingCenter, Good::Grain) => 2, // per unit
                        (BuildingKind::FoodProcessingCenter, Good::Fruit) => 1,
                        (BuildingKind::FoodProcessingCenter, Good::Livestock) => 1,
                        (BuildingKind::FoodProcessingCenter, Good::Fish) => 1,
                        _ => 0,
                    };
                    count * per_unit_cost
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
    allocations: Query<&Allocations, Changed<Allocations>>,
    mut summaries: Query<(&mut Text, &AllocationSummary)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    if let Ok(alloc) = allocations.get(player.0) {
        for (mut text, summary) in summaries.iter_mut() {
            text.0 = match summary.allocation_type {
                AllocationType::Recruitment => {
                    let allocated = alloc.recruitment_count();
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
                    let allocated = alloc.training_count(from_skill);
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
                    let allocated = alloc.production_count(building_entity, output_good);
                    if allocated > 0 {
                        format!("→ Will produce {} {:?} next turn", allocated, output_good)
                    } else {
                        "→ No production planned".to_string()
                    }
                }
            };
        }
    }
}
