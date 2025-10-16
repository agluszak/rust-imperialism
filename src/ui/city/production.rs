use bevy::prelude::*;

use super::components::ProductionChoiceButton;

/// Handle production choice button clicks (DEPRECATED - choices are now automatic)
/// This system is no longer active; inputs are chosen automatically based on availability
pub fn handle_production_choice_buttons(
    _interactions: Query<(&Interaction, &ProductionChoiceButton), Changed<Interaction>>,
    _player_nation: Option<Res<crate::economy::PlayerNation>>,
    _allocations: Query<&crate::economy::Allocations>,
    _prod_writer: MessageWriter<crate::economy::AdjustProduction>,
) {
    // DEPRECATED: ProductionChoice buttons removed - inputs now chosen automatically
}
