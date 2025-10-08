use bevy::prelude::*;

use super::components::{AdjustProductionButton, ChangeProductionSettings, ProductionChoiceButton};

/// Handle production choice button clicks
pub fn handle_production_choice_buttons(
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
pub fn handle_adjust_production_buttons(
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

/// Apply production settings changes with resource reservation
pub fn apply_production_settings_changes(
    mut change_events: MessageReader<ChangeProductionSettings>,
    mut query: Query<(
        &mut crate::economy::production::ProductionSettings,
        &crate::economy::Building,
        &mut crate::economy::Stockpile,
    )>,
) {
    for event in change_events.read() {
        if let Ok((mut settings, building, mut stockpile)) = query.get_mut(event.building_entity) {
            // Calculate old input requirements and unreserve them
            let old_inputs =
                calculate_inputs(building.kind, settings.choice, settings.target_output);
            for (good, qty) in old_inputs {
                stockpile.unreserve(good, qty);
            }

            // Apply choice change
            if let Some(new_choice) = event.new_choice {
                settings.choice = new_choice;
                info!("Changed production choice to {:?}", new_choice);
            }

            // Apply target delta
            if let Some(delta) = event.target_delta {
                let new_target = (settings.target_output as i32 + delta).max(0) as u32;
                settings.target_output = new_target.min(building.capacity);
                info!("Adjusted production target to {}", settings.target_output);
            }

            // Calculate new input requirements
            let new_inputs =
                calculate_inputs(building.kind, settings.choice, settings.target_output);

            // Try to reserve new inputs
            let mut can_reserve = true;
            for (good, qty) in &new_inputs {
                if !stockpile.has_available(*good, *qty) {
                    can_reserve = false;
                    break;
                }
            }

            if can_reserve {
                // Reserve all inputs
                for (good, qty) in new_inputs {
                    stockpile.reserve(good, qty);
                }
            } else {
                // Can't reserve all - reduce target to what's possible
                let max_possible = calculate_max_possible_output(
                    building.kind,
                    settings.choice,
                    building.capacity,
                    &stockpile,
                );
                settings.target_output = max_possible;
                info!(
                    "Reduced target to {} due to insufficient inputs",
                    max_possible
                );

                // Reserve what we can
                let reduced_inputs =
                    calculate_inputs(building.kind, settings.choice, settings.target_output);
                for (good, qty) in reduced_inputs {
                    stockpile.reserve(good, qty);
                }
            }
        }
    }
}

/// Calculate input requirements for a building given its kind, choice, and target output
fn calculate_inputs(
    kind: crate::economy::production::BuildingKind,
    choice: crate::economy::production::ProductionChoice,
    target: u32,
) -> Vec<(crate::economy::Good, u32)> {
    use crate::economy::Good;
    use crate::economy::production::{BuildingKind, ProductionChoice};

    match kind {
        BuildingKind::TextileMill => {
            let input = match choice {
                ProductionChoice::UseCotton => Good::Cotton,
                ProductionChoice::UseWool => Good::Wool,
                _ => Good::Cotton,
            };
            vec![(input, target * 2)] // 2:1 ratio
        }
        BuildingKind::LumberMill => {
            vec![(Good::Timber, target * 2)] // 2:1 ratio
        }
        BuildingKind::SteelMill => {
            vec![(Good::Iron, target), (Good::Coal, target)] // 1:1 ratio for each
        }
        BuildingKind::FoodProcessingCenter => {
            let meat = match choice {
                ProductionChoice::UseLivestock => Good::Livestock,
                ProductionChoice::UseFish => Good::Fish,
                _ => Good::Livestock,
            };
            vec![
                (Good::Grain, target * 2),
                (Good::Fruit, target),
                (meat, target),
            ]
        }
        _ => vec![], // Non-production buildings
    }
}

/// Calculate maximum possible output given available resources
fn calculate_max_possible_output(
    kind: crate::economy::production::BuildingKind,
    choice: crate::economy::production::ProductionChoice,
    capacity: u32,
    stockpile: &crate::economy::Stockpile,
) -> u32 {
    use crate::economy::Good;
    use crate::economy::production::{BuildingKind, ProductionChoice};

    match kind {
        BuildingKind::TextileMill => {
            let input = match choice {
                ProductionChoice::UseCotton => Good::Cotton,
                ProductionChoice::UseWool => Good::Wool,
                _ => Good::Cotton,
            };
            let available = stockpile.get_available(input);
            (available / 2).min(capacity) // 2:1 ratio
        }
        BuildingKind::LumberMill => {
            let available = stockpile.get_available(Good::Timber);
            (available / 2).min(capacity) // 2:1 ratio
        }
        BuildingKind::SteelMill => {
            let iron = stockpile.get_available(Good::Iron);
            let coal = stockpile.get_available(Good::Coal);
            iron.min(coal).min(capacity) // 1:1 ratio, limited by both
        }
        BuildingKind::FoodProcessingCenter => {
            let meat = match choice {
                ProductionChoice::UseLivestock => Good::Livestock,
                ProductionChoice::UseFish => Good::Fish,
                _ => Good::Livestock,
            };
            let grain = stockpile.get_available(Good::Grain) / 2;
            let fruit = stockpile.get_available(Good::Fruit);
            let meat_qty = stockpile.get_available(meat);
            grain.min(fruit).min(meat_qty).min(capacity)
        }
        _ => 0, // Non-production buildings
    }
}

/// Initialize reservations for existing production settings (runs at startup)
pub fn initialize_production_reservations(
    mut query: Query<
        (
            &crate::economy::production::ProductionSettings,
            &crate::economy::Building,
            &mut crate::economy::Stockpile,
        ),
        Added<crate::economy::production::ProductionSettings>,
    >,
) {
    for (settings, building, mut stockpile) in query.iter_mut() {
        if settings.target_output > 0 {
            let inputs = calculate_inputs(building.kind, settings.choice, settings.target_output);
            for (good, qty) in inputs {
                if !stockpile.reserve(good, qty) {
                    info!(
                        "Could not reserve {} units of {:?} for initial production settings",
                        qty, good
                    );
                }
            }
        }
    }
}

/// Update building panels (placeholder for now - we'll implement dynamic updates if needed)
pub fn update_building_panels() {
    // Buildings are static for now, but we could add dynamic updates here
}
