// Dialog modules
pub mod production;
pub mod special;
pub mod systems;
pub mod types;
pub mod window;

// Re-export key types and functions
pub use production::{populate_production_dialog, update_production_labor_display};
pub use special::{
    populate_special_dialog, update_capitol_capacity_display, update_capitol_requirement_displays,
    update_trade_school_paper_display, update_trade_school_workforce_display,
};
pub use systems::{close_building_dialogs, open_building_dialogs};
pub use types::{
    BuildingDialog, CloseBuildingDialog, DialogCloseButton, DialogContentArea, DialogZIndexCounter,
    OpenBuildingDialog,
};
pub use window::{handle_dialog_close_buttons, spawn_dialog_frame, update_close_button_visuals};
