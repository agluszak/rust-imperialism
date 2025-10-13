use bevy::prelude::*;

use super::components::ProductionChoiceButton;

/// Handle production choice button clicks (updates allocation with new choice)
pub fn handle_production_choice_buttons(
    interactions: Query<(&Interaction, &ProductionChoiceButton), Changed<Interaction>>,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    allocations: Query<&crate::economy::Allocations>,
    mut prod_writer: MessageWriter<crate::economy::AdjustProduction>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(alloc) = allocations.get(player.0) else {
        return;
    };

    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            info!("Production choice button clicked: {:?}", button.choice);

            // For now, just write the choice change with zero output
            // The actual output allocation will be handled by the +/- buttons
            // TODO: This needs to know which Good the button corresponds to
            // For now, default to the first possible output for the building kind
            let output_good = match button.choice {
                crate::economy::production::ProductionChoice::UseCotton
                | crate::economy::production::ProductionChoice::UseWool => {
                    crate::economy::Good::Fabric
                }
                crate::economy::production::ProductionChoice::MakeLumber => {
                    crate::economy::Good::Lumber
                }
                crate::economy::production::ProductionChoice::MakePaper => {
                    crate::economy::Good::Paper
                }
                crate::economy::production::ProductionChoice::UseLivestock
                | crate::economy::production::ProductionChoice::UseFish => {
                    crate::economy::Good::CannedFood
                }
            };

            // Get current target output for this specific good
            let current_target = alloc.production_count(button.building_entity, output_good) as u32;

            // Write AdjustProduction with new choice but keep current target
            prod_writer.write(crate::economy::AdjustProduction {
                nation: player.0,
                building: button.building_entity,
                output_good,
                choice: Some(button.choice),
                target_output: current_target,
            });
        }
    }
}
