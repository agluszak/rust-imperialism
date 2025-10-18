use crate::ui::menu::AppState;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

/// Global game mode state: strategic views
#[derive(SubStates, Debug, Clone, Eq, PartialEq, Hash, Default, Reflect)]
#[source(AppState = AppState::InGame)]
pub enum GameMode {
    /// Hex map view
    #[default]
    Map,
    /// City/Production management view
    City,
    /// Transport/logistics tools
    Transport,
    /// World market screen
    Market,
    /// Diplomacy/influence screen
    Diplomacy,
}

/// Creates an observer that switches to the specified mode when button is activated
pub fn switch_to_mode(mode: GameMode) -> impl Bundle {
    observe(
        move |_activate: On<Activate>, mut next_state: ResMut<NextState<GameMode>>| {
            next_state.set(mode.clone());
        },
    )
}
