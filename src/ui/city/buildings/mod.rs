// Building grid modules
pub mod buttons;
pub mod grid;

// Re-export spawn and update functions
pub use buttons::{handle_building_button_clicks, update_building_button_visuals};
pub use grid::{spawn_building_grid, update_building_buttons};
