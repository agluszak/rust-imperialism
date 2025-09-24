use bevy::prelude::*;

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
pub struct ScrollbarThumb;

#[derive(Component)]
pub struct ScrollbarTrack;

#[derive(Component)]
pub struct ScrollbarDragStart {
    pub position: Vec2,
    pub _scroll_position: Vec2,
}
