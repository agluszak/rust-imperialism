//! Game constants and configuration values
//!
//! This module centralizes all magic numbers and configuration values used throughout the game.

/// Default tile size (width x height in pixels) - matches Imperialism assets
pub const TILE_SIZE: f32 = 64.0;

/// Default map size (width x height in tiles)
pub const MAP_SIZE: u32 = 32;

/// Map generation seed for terrain generator
pub const TERRAIN_SEED: u32 = 12345;

/// Get the grid size for hexagonal tilemap
/// Slightly increase vertical spacing so full square tiles are visible
pub fn get_hex_grid_size() -> bevy_ecs_tilemap::prelude::TilemapGridSize {
    bevy_ecs_tilemap::prelude::TilemapGridSize {
        x: TILE_SIZE,
        y: TILE_SIZE * 1.3,
    }
}
