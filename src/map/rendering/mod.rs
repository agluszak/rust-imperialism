// Rendering modules for map elements
pub mod border_rendering;
pub mod city_rendering;
pub mod improvement_rendering;
pub mod map_visual;
pub mod prospecting_markers;
pub mod terrain_atlas;
pub mod transport_debug;
pub mod transport_rendering;

use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;
use bevy::prelude::*;

// Re-exports for convenience
pub use border_rendering::*;
pub use city_rendering::*;
pub use improvement_rendering::*;
pub use map_visual::*;
pub use prospecting_markers::*;
pub use terrain_atlas::*;
pub use transport_debug::*;
pub use transport_rendering::*;

/// Unified plugin for all map-related rendering
pub struct MapRenderingPlugin;

impl Plugin for MapRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Register loader
        app.register_asset_loader(crate::bmp_loader::ImperialismBmpLoader);

        // Register resources
        app.init_resource::<improvement_rendering::ConnectivityOverlaySettings>()
            .init_resource::<transport_debug::TransportDebugSettings>()
            .init_resource::<transport_debug::TransportDebugFont>()
            .init_resource::<transport_rendering::HoveredTile>();

        // Terrain atlas loading
        app.add_systems(Startup, terrain_atlas::start_terrain_atlas_loading)
            .add_systems(Update, terrain_atlas::build_terrain_atlas_when_ready);

        // Map setup rendering
        app.add_systems(
            Update,
            crate::map::setup_tilemap_rendering.run_if(in_state(AppState::InGame)),
        );

        // Core map rendering systems
        app.add_systems(
            Update,
            (
                border_rendering::render_borders,
                city_rendering::render_city_visuals,
                city_rendering::update_city_visual_positions,
                improvement_rendering::render_improvement_markers,
                improvement_rendering::update_improvement_markers,
                improvement_rendering::cleanup_removed_improvement_markers,
                improvement_rendering::toggle_connectivity_overlay,
                improvement_rendering::update_connectivity_overlay,
                prospecting_markers::render_prospected_empty_markers,
                prospecting_markers::render_prospected_mineral_markers,
                transport_rendering::render_rails,
                transport_rendering::update_depot_visuals,
                transport_rendering::update_port_visuals,
                transport_rendering::render_shadow_rail,
                transport_debug::toggle_transport_debug,
                transport_debug::render_transport_debug,
                crate::civilians::rendering::render_civilian_visuals,
                crate::civilians::rendering::update_civilian_visual_colors,
            )
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameMode::Map)),
        );
    }
}
