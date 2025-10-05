use bevy::prelude::*;

// UI marker components
#[derive(Component)]
pub struct TurnDisplay;

#[derive(Component)]
pub struct TerminalWindow;

#[derive(Component)]
pub struct TerminalOutput;

#[derive(Component)]
pub struct ScrollableTerminal;

/// Marker for the root entities of gameplay UI (HUD, terminal, sidebar)
#[derive(Component)]
pub struct GameplayUIRoot;


/// Marker for calendar text in HUD
#[derive(Component)]
pub struct CalendarDisplay;

/// Marker for treasury text in HUD
#[derive(Component)]
pub struct TreasuryDisplay;

/// Marker for tilemap entities that should only be visible in Map mode
#[derive(Component)]
pub struct MapTilemap;
