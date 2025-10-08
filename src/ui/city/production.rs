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

/// Apply production settings changes
pub fn apply_production_settings_changes(
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

/// Update building panels (placeholder for now - we'll implement dynamic updates if needed)
pub fn update_building_panels() {
    // Buildings are static for now, but we could add dynamic updates here
}
