use bevy::prelude::*;

use crate::economy::{Calendar, PlayerNation, Treasury};
use crate::ui::components::{CalendarDisplay, TreasuryDisplay, TurnDisplay};
use crate::ui::state::{UIState, UIStateUpdated};

/// Update turn display using centralized UI state
/// This system only runs when UI state has actually changed, reducing overhead
pub fn update_turn_display(
    mut state_events: MessageReader<UIStateUpdated>,
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

/// Update calendar HUD text when calendar changes or on first frame
pub fn update_calendar_display(
    calendar: Option<Res<Calendar>>,
    mut q: Query<&mut Text, With<CalendarDisplay>>,
) {
    if let Some(calendar) = calendar {
        if calendar.is_changed() || calendar.is_added() {
            for mut text in q.iter_mut() {
                text.0 = calendar.display();
            }
        }
    }
}

fn format_currency(value: i64) -> String {
    // naive thousands separator with commas
    let mut s = value.abs().to_string();
    let mut i = s.len() as isize - 3;
    while i > 0 {
        s.insert(i as usize, ',');
        i -= 3;
    }
    if value < 0 {
        format!("-${}", s)
    } else {
        format!("${}", s)
    }
}

/// Update treasury HUD text based on the active player's nation
pub fn update_treasury_display(
    player: Option<Res<PlayerNation>>,
    treasuries: Query<&Treasury>,
    mut q: Query<&mut Text, With<TreasuryDisplay>>,
) {
    if let Some(player) = player {
        if let Ok(treasury) = treasuries.get(player.0) {
            let s = format_currency(treasury.0);
            for mut text in q.iter_mut() {
                text.0 = s.clone();
            }
        }
    }
}
