use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

// UI marker components
#[derive(Component)]
pub struct TurnDisplay;

#[derive(Component)]
pub struct HeroStatusDisplay;

#[derive(Component)]
pub struct TerminalWindow;

#[derive(Component)]
pub struct TerminalOutput;

#[derive(Component)]
pub struct ScrollableTerminal;

#[derive(Component)]
pub struct Scrollbar;

#[derive(Component)]
pub struct ScrollbarThumb;

#[derive(Component)]
pub struct ScrollbarTrack;

#[derive(Component)]
pub struct ScrollbarDragStart {
    pub position: Vec2,
    pub scroll_position: Vec2,
}

// Convenience type export (used in several systems)
pub type CursorPos = RelativeCursorPosition;
