use bevy::prelude::*;

// UI marker components
#[derive(Component)]
pub struct TurnDisplay;

/// Marker for the root entities of gameplay UI (HUD, sidebar)
#[derive(Component)]
pub struct GameplayUIRoot;

/// Marker for calendar text in HUD
#[derive(Component)]
pub struct CalendarDisplay;

/// Marker for treasury text in HUD
#[derive(Component)]
pub struct TreasuryDisplay;

/// Marker for tilemap entities that should only be visible in Map mode
#[derive(Component, Default)]
pub struct MapTilemap;

/// Marker for tile info display showing hovered tile information
#[derive(Component)]
pub struct TileInfoDisplay;
