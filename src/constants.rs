//! Game constants and configuration values
//!
//! This module centralizes all magic numbers and configuration values used throughout the game.

use bevy::prelude::*;

// ============================================================================
// COMBAT CONSTANTS
// ============================================================================

/// Action point cost for a single attack
pub const ATTACK_ACTION_COST: u32 = 1;

/// Default hero attack damage (used as fallback)
pub const DEFAULT_HERO_DAMAGE: u32 = 3;

/// Hero attack damage configured in spawn
pub const HERO_DAMAGE: u32 = 25;

/// Default monster attack damage
pub const MONSTER_DAMAGE: u32 = 2;

// ============================================================================
// ACTION POINTS
// ============================================================================

/// Maximum action points for heroes
pub const HERO_MAX_ACTION_POINTS: u32 = 6;

/// Maximum action points for monsters
pub const MONSTER_MAX_ACTION_POINTS: u32 = 4;

// ============================================================================
// HEALTH CONSTANTS
// ============================================================================

/// Default hero maximum health
pub const HERO_MAX_HEALTH: u32 = 100;

/// Default monster maximum health
pub const MONSTER_MAX_HEALTH: u32 = 3;

/// Health percentage threshold for low health behavior (25%)
pub const LOW_HEALTH_THRESHOLD_PERCENT: u32 = 25;

/// Number of kills required for hero to heal
pub const KILLS_PER_HEAL: u32 = 3;

// ============================================================================
// MONSTER AI CONSTANTS
// ============================================================================

/// Default sight range for monsters (in tiles)
pub const MONSTER_SIGHT_RANGE: u32 = 5;

/// Maximum number of monsters allowed on map at once
pub const MAX_MONSTERS: usize = 5;

/// Spawn a new monster every N turns
pub const MONSTER_SPAWN_INTERVAL: u32 = 3;

/// Distance at which monsters will attack (adjacent tiles)
pub const MONSTER_ATTACK_RANGE: u32 = 1;

// ============================================================================
// MOVEMENT CONSTANTS
// ============================================================================

/// Movement speed for heroes (pixels per second)
pub const HERO_MOVEMENT_SPEED: f32 = 200.0;

/// Movement speed for monsters (pixels per second)
pub const MONSTER_MOVEMENT_SPEED: f32 = 150.0;

/// Distance threshold to consider entity has reached target
pub const MOVEMENT_ARRIVAL_THRESHOLD: f32 = 5.0;

/// High cost value for impassable tiles (used in pathfinding)
pub const IMPASSABLE_TILE_COST: f32 = 999.0;

// ============================================================================
// VISUAL CONSTANTS - Colors
// ============================================================================

/// Hero sprite color when not selected
pub const HERO_COLOR_NORMAL: Color = Color::srgb(0.0, 0.0, 1.0); // Blue

/// Hero sprite color when selected
pub const HERO_COLOR_SELECTED: Color = Color::srgb(1.0, 1.0, 0.0); // Yellow

/// Monster sprite color
pub const MONSTER_COLOR: Color = Color::srgb(1.0, 0.0, 0.0); // Red

/// Path preview color for reachable waypoints
pub const PATH_PREVIEW_REACHABLE: Color = Color::srgba(1.0, 1.0, 0.0, 0.5); // Yellow, semi-transparent

/// Path preview color for unreachable waypoints
pub const PATH_PREVIEW_UNREACHABLE: Color = Color::srgba(1.0, 0.0, 0.0, 0.5); // Red, semi-transparent

/// Target marker color when reachable
pub const TARGET_MARKER_REACHABLE: Color = Color::srgba(0.0, 1.0, 0.0, 0.7); // Green, semi-transparent

/// Target marker color when unreachable
pub const TARGET_MARKER_UNREACHABLE: Color = Color::srgba(1.0, 0.5, 0.0, 0.7); // Orange, semi-transparent

// ============================================================================
// VISUAL CONSTANTS - Sizes
// ============================================================================

/// Size of hero sprite (width x height in pixels)
pub const HERO_SPRITE_SIZE: Vec2 = Vec2::new(16.0, 16.0);

/// Size of monster sprite (width x height in pixels)
pub const MONSTER_SPRITE_SIZE: Vec2 = Vec2::new(10.0, 10.0);

/// Size of path preview waypoint markers
pub const PATH_WAYPOINT_SIZE: Vec2 = Vec2::new(4.0, 4.0);

/// Size of target marker
pub const TARGET_MARKER_SIZE: Vec2 = Vec2::new(8.0, 8.0);

// ============================================================================
// VISUAL CONSTANTS - Z-Layers
// ============================================================================

/// Z-coordinate for tiles (background layer)
pub const Z_LAYER_TILES: f32 = 0.0;

/// Z-coordinate for monsters
pub const Z_LAYER_MONSTERS: f32 = 1.0;

/// Z-coordinate for heroes and UI overlays
pub const Z_LAYER_HEROES: f32 = 2.0;

/// Z-coordinate for path preview markers
pub const Z_LAYER_PATH_PREVIEW: f32 = 2.0;

// ============================================================================
// MAP CONSTANTS
// ============================================================================

/// Default tile size (width x height in pixels)
pub const TILE_SIZE: f32 = 16.0;

/// Default map size (width x height in tiles)
pub const MAP_SIZE: u32 = 32;

/// Map generation seed for terrain generator
pub const TERRAIN_SEED: u32 = 12345;

// ============================================================================
// SPAWN POSITIONS
// ============================================================================

/// Hero spawn position on 32x32 map (center)
pub const HERO_SPAWN_X: u32 = 16;
pub const HERO_SPAWN_Y: u32 = 16;
