use bevy::prelude::*;

// Re-exports for public API
pub use components::*;
pub use layout::{ensure_city_screen_visible, hide_city_screen};

// Module declarations
pub mod allocation_ui_unified; // Unified allocation UI systems
pub mod allocation_widgets; // Reusable allocation widgets
pub mod buildings;
pub mod components;
pub mod dialogs;
pub mod hud;
pub mod layout;
pub mod production;
pub mod workforce;

// No private imports needed - using fully qualified paths in plugin registration

/// Plugin that manages City Mode UI
pub struct CityUIPlugin;

impl Plugin for CityUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<dialogs::DialogZIndexCounter>()
            .add_message::<HireCivilian>()
            .add_message::<crate::economy::RecruitWorkers>()
            .add_message::<crate::economy::TrainWorker>()
            .add_message::<dialogs::OpenBuildingDialog>()
            .add_message::<dialogs::CloseBuildingDialog>()
            .add_message::<crate::economy::AdjustRecruitment>()
            .add_message::<crate::economy::AdjustTraining>()
            .add_message::<crate::economy::AdjustProduction>()
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
                    // HUD update systems
                    hud::update_labor_display,
                    hud::update_workforce_display,
                    hud::update_food_demand_display,
                    hud::update_warehouse_display,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::City)),
            )
            .add_systems(
                Update,
                (
                    // Building grid systems
                    buildings::update_building_buttons,
                    buildings::handle_building_button_clicks,
                    buildings::update_building_button_visuals,
                    // Dialog systems
                    dialogs::open_building_dialogs,
                    dialogs::close_building_dialogs,
                    dialogs::handle_dialog_close_buttons,
                    dialogs::update_close_button_visuals,
                    // Dialog content population
                    dialogs::populate_production_dialog,
                    dialogs::populate_special_dialog,
                    // Dialog content updates
                    dialogs::update_production_labor_display,
                    dialogs::update_capitol_requirement_displays,
                    dialogs::update_capitol_capacity_display,
                    dialogs::update_trade_school_workforce_display,
                    dialogs::update_trade_school_paper_display,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::City)),
            )
            .add_systems(
                Update,
                (
                    // Input handlers
                    workforce::handle_hire_button_clicks,
                    workforce::spawn_hired_civilian,
                    production::handle_production_choice_buttons,
                    workforce::handle_recruit_workers_buttons,
                    workforce::handle_train_worker_buttons,
                    allocation_ui_unified::handle_all_stepper_buttons,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::City)),
            )
            .add_systems(
                Update,
                (
                    // Unified allocation UI rendering systems
                    allocation_ui_unified::update_all_stepper_displays,
                    allocation_ui_unified::update_all_allocation_bars,
                    allocation_ui_unified::update_all_allocation_summaries,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::City)),
            );
    }
}
