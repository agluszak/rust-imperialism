use crate::ui::menu::AppState;
use bevy::prelude::*;

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

/// Marker for UI button that switches to City mode
#[derive(Component)]
pub struct CityModeButton;

/// Marker for UI button that switches to Map mode
#[derive(Component)]
pub struct MapModeButton;

/// Marker for UI button that switches to Transport mode
#[derive(Component)]
pub struct TransportModeButton;

/// Marker for UI button that switches to Market mode
#[derive(Component)]
pub struct MarketModeButton;

/// Marker for UI button that switches to Diplomacy mode
#[derive(Component)]
pub struct DiplomacyModeButton;

/// Handle clicks on the mode buttons
pub fn handle_mode_buttons(
    mut interactions: Query<
        (
            &Interaction,
            Option<&CityModeButton>,
            Option<&MapModeButton>,
            Option<&TransportModeButton>,
            Option<&MarketModeButton>,
            Option<&DiplomacyModeButton>,
        ),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameMode>>,
) {
    for (interaction, is_city_btn, is_map_btn, is_transport_btn, is_market_btn, is_diplomacy_btn) in
        interactions.iter_mut()
    {
        if *interaction == Interaction::Pressed {
            if is_city_btn.is_some() {
                next_state.set(GameMode::City);
            } else if is_transport_btn.is_some() {
                next_state.set(GameMode::Transport);
            } else if is_market_btn.is_some() {
                next_state.set(GameMode::Market);
            } else if is_diplomacy_btn.is_some() {
                next_state.set(GameMode::Diplomacy);
            } else if is_map_btn.is_some() {
                next_state.set(GameMode::Map);
            }
        }
    }
}
