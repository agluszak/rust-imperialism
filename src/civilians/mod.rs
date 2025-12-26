use bevy::prelude::*;

use crate::turn_system::{PlayerTurnSet, TurnPhase};

// Re-exports for public API
pub use crate::messages::civilians::{
    CivilianCommand, CivilianCommandError, CivilianCommandRejected, HireCivilian,
};
pub use commands::*;
pub use jobs::{advance_civilian_jobs, complete_improvement_jobs, reset_civilian_actions};
pub use types::*;

// Module declarations
pub mod commands;
pub mod engineering;
pub mod hiring;
pub mod jobs;
pub mod order_validation;
pub mod rendering;
pub mod systems;
pub mod types;
pub mod ui_components;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod player_ownership_test;

#[cfg(test)]
mod ui_ownership_test;

// No private imports needed - using fully qualified paths in plugin registration

/// System set for civilian job processing during turn start.
/// Runs within PlayerTurnSet::Maintenance.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum CivilianJobSet {
    /// Advance job timers (decrement turns_remaining)
    Advance,
    /// Complete jobs that finished (turns_remaining == 0)
    Complete,
    /// Reset civilian movement flags for new turn
    Reset,
}

/// Core civilian gameplay plugin (logic, no rendering).
/// Use this in headless tests.
pub struct CivilianPlugin;

impl Plugin for CivilianPlugin {
    fn build(&self, app: &mut App) {
        // Configure civilian job set ordering
        app.configure_sets(
            OnEnter(TurnPhase::PlayerTurn),
            (
                CivilianJobSet::Advance,
                CivilianJobSet::Complete,
                CivilianJobSet::Reset,
            )
                .chain()
                .in_set(PlayerTurnSet::Maintenance),
        );

        app.init_resource::<crate::civilians::types::ProspectingKnowledge>()
            .init_resource::<crate::civilians::types::NextCivilianId>()
            .init_resource::<SelectedCivilian>()
            .add_message::<SelectCivilian>()
            .add_message::<CivilianCommand>()
            .add_message::<CivilianCommandRejected>()
            .add_message::<DeselectCivilian>()
            .add_message::<RescindOrders>()
            .add_message::<HireCivilian>()
            // Selection handler runs always to react to events immediately
            .add_systems(
                Update,
                (
                    systems::handle_civilian_selection,
                    systems::handle_deselect_key,
                    systems::handle_deselection,
                ),
            )
            .add_systems(
                Update,
                hiring::spawn_hired_civilian.run_if(in_state(crate::ui::menu::AppState::InGame)),
            )
            .add_systems(
                Update,
                systems::handle_rescind_orders
                    .before(systems::handle_civilian_commands)
                    .run_if(in_state(crate::ui::mode::GameMode::Map)),
            )
            .add_systems(
                Update,
                (
                    systems::handle_civilian_commands,
                    // Apply deferred commands so CivilianOrder is visible to execution systems
                    bevy::ecs::schedule::ApplyDeferred,
                    systems::execute_move_orders,
                    systems::execute_skip_and_sleep_orders,
                    engineering::execute_engineer_orders,
                    engineering::execute_prospector_orders,
                    engineering::execute_civilian_improvement_orders,
                    ui_components::update_civilian_orders_ui,
                    ui_components::update_rescind_orders_ui,
                )
                    .chain()
                    .run_if(in_state(crate::ui::mode::GameMode::Map)),
            );

        // ====================================================================
        // Turn-based systems (run once on PlayerTurn entry)
        // ====================================================================
        // Order: advance first (decrements counter), then complete (checks for 0 and applies effect)
        // The advance system removes CivilianJob via deferred commands, so complete must see it
        // before the removal is applied

        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            jobs::advance_civilian_jobs.in_set(CivilianJobSet::Advance),
        );

        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            jobs::complete_improvement_jobs.in_set(CivilianJobSet::Complete),
        );

        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            jobs::reset_civilian_actions.in_set(CivilianJobSet::Reset),
        );
    }
}

/// Civilian rendering plugin (sprites and visual updates).
/// Requires AssetServer and should not be added in headless tests.
pub struct CivilianRenderingPlugin;

impl Plugin for CivilianRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                rendering::render_civilian_visuals,
                rendering::update_civilian_visual_colors,
            )
                .chain()
                .run_if(in_state(crate::ui::mode::GameMode::Map)),
        );
    }
}
