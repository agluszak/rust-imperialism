use bevy::prelude::*;

// Re-exports for public API
pub use components::*;
pub use layout::{ensure_city_screen_visible, hide_city_screen};

// Module declarations
pub mod components;
pub mod layout;
pub mod production;
pub mod warehouse;
pub mod workforce;

// No private imports needed - using fully qualified paths in plugin registration

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HireCivilian>()
            .add_message::<ChangeProductionSettings>()
            .add_message::<crate::economy::RecruitWorkers>()
            .add_message::<crate::economy::TrainWorker>()
            .add_systems(
                OnEnter(crate::ui::mode::GameMode::City),
                layout::ensure_city_screen_visible,
            )
            .add_systems(
                OnExit(crate::ui::mode::GameMode::City),
                layout::hide_city_screen,
            )
            .add_systems(
                Update,
                (
                    workforce::handle_hire_button_clicks,
                    workforce::spawn_hired_civilian,
                    production::handle_production_choice_buttons,
                    production::handle_adjust_production_buttons,
                    production::apply_production_settings_changes,
                    workforce::handle_recruit_workers_buttons,
                    workforce::handle_train_worker_buttons,
                    production::update_building_panels,
                    workforce::update_workforce_panel,
                    warehouse::update_stockpile_display,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::City)),
            );
    }
}
