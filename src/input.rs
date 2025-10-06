use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, _app: &mut App) {
        // Terrain editing and transport tile clicking removed
        // Infrastructure now built by Engineer units
    }
}

// Main input dispatcher - converts clicks to strategy-friendly events
pub fn handle_tile_click(_trigger: On<Pointer<Click>>) {
    // TODO: Add Engineer unit selection here when clicking on civilian units
}
