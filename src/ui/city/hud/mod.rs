// HUD border modules
pub mod food;
pub mod labor;
pub mod province_resources;
pub mod warehouse;

// Re-export spawn functions
pub use food::spawn_food_demand_panel;
pub use labor::spawn_labor_pool_panel;
pub use province_resources::spawn_province_resources_panel;
pub use warehouse::spawn_warehouse_hud;

// Re-export update systems
pub use food::update_food_demand_display;
pub use labor::{update_labor_display, update_workforce_display};
pub use province_resources::update_province_resources_display;
pub use warehouse::update_warehouse_display;
