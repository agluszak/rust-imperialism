use bevy::prelude::*;

// Re-exports for public API
pub use crate::messages::civilians::{
    CivilianCommand, CivilianCommandError, CivilianCommandRejected,
};
pub use commands::*;
pub use jobs::{advance_civilian_jobs, complete_improvement_jobs, reset_civilian_actions};
pub use types::*;

use crate::ui::mode::GameMode;

// Module declarations
pub mod commands;
pub mod engineering;
pub mod jobs;
pub mod order_validation;
pub mod rendering;
pub mod systems;
pub mod types;
pub mod ui_components;

#[cfg(test)]
mod tests;

// No private imports needed - using fully qualified paths in plugin registration

pub struct CivilianPlugin;

impl Plugin for CivilianPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SelectCivilian>()
            .add_message::<CivilianCommand>()
            .add_message::<CivilianCommandRejected>()
            .add_message::<DeselectCivilian>()
            .add_message::<DeselectAllCivilians>()
            .add_message::<RescindOrders>()
            // Selection handler runs always to react to events immediately
            .add_systems(
                Update,
                (
                    systems::handle_civilian_selection,
                    systems::handle_deselect_key,
                    systems::handle_deselection,
                    systems::handle_deselect_all,
                ),
            )
            .add_systems(
                Update,
                (
                    systems::handle_civilian_commands,
                    systems::execute_move_orders,
                    engineering::execute_engineer_orders,
                    engineering::execute_prospector_orders,
                    engineering::execute_civilian_improvement_orders,
                    systems::handle_rescind_orders,
                    ui_components::update_engineer_orders_ui,
                    ui_components::update_improver_orders_ui,
                    ui_components::update_rescind_orders_ui,
                    rendering::render_civilian_visuals,
                    rendering::update_civilian_visual_colors,
                )
                    .run_if(in_state(GameMode::Map)),
            );
    }
}
