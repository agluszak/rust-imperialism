use bevy::prelude::*;

use crate::ui::components::{HeroStatusDisplay, MonsterCountDisplay, TurnDisplay};
use crate::ui::state::{UIState, UIStateUpdated};

/// Update turn display using centralized UI state
/// This system only runs when UI state has actually changed, reducing overhead
pub fn update_turn_display(
    mut state_events: EventReader<UIStateUpdated>,
    ui_state: Res<UIState>,
    mut query: Query<&mut Text, With<TurnDisplay>>,
) {
    // Only update when state has changed
    if !state_events.is_empty() {
        state_events.clear(); // Consume all events

        for mut text in query.iter_mut() {
            text.0 = ui_state.turn_display_text();
        }
    }
}

/// Update hero status display using centralized UI state
/// This system only runs when UI state has actually changed, reducing overhead
pub fn update_hero_status_display(
    mut state_events: EventReader<UIStateUpdated>,
    ui_state: Res<UIState>,
    mut text_query: Query<&mut Text, With<HeroStatusDisplay>>,
) {
    // Only update when state has changed
    if !state_events.is_empty() {
        state_events.clear(); // Consume all events

        for mut text in text_query.iter_mut() {
            text.0 = ui_state.hero_status_text();
        }
    }
}

/// Update monster count display using centralized UI state
/// This demonstrates how easy it is to add new UI elements with centralized state management
pub fn update_monster_count_display(
    mut state_events: EventReader<UIStateUpdated>,
    ui_state: Res<UIState>,
    mut text_query: Query<&mut Text, With<MonsterCountDisplay>>,
) {
    // Only update when state has changed
    if !state_events.is_empty() {
        state_events.clear(); // Consume all events

        for mut text in text_query.iter_mut() {
            text.0 = ui_state.monster_count_text();
        }
    }
}
