// Rendering modules for map elements
pub mod border_rendering;
pub mod city_rendering;
pub mod connected_resource_debug;
pub mod improvement_rendering;
pub mod map_visual;
pub mod prospecting_markers;
pub mod terrain_atlas;
pub mod transport_debug;
pub mod transport_rendering;

// Re-exports for convenience
pub use border_rendering::*;
pub use city_rendering::*;
pub use connected_resource_debug::*;
pub use improvement_rendering::*;
pub use map_visual::*;
pub use prospecting_markers::*;
pub use terrain_atlas::*;
pub use transport_debug::*;
pub use transport_rendering::*;
