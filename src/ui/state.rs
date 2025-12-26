use crate::turn_system::{TurnCounter, TurnPhase};
use bevy::prelude::*;

/// Centralized UI state that consolidates all game state needed by UI systems
/// This reduces the number of queries each UI system needs to perform
#[derive(Resource, Default, Debug)]
pub struct UIState {
    pub turn: TurnState,
}

#[derive(Debug, Clone)]
pub struct TurnState {
    pub current_turn: u32,
    pub phase: TurnPhase,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            current_turn: 1,
            phase: TurnPhase::PlayerTurn,
        }
    }
}

impl UIState {
    /// Update UI state from turn counter and phase
    pub fn update(&mut self, turn: u32, phase: TurnPhase) {
        self.turn.current_turn = turn;
        self.turn.phase = phase;
    }

    /// Check if any UI-relevant state has changed
    pub fn needs_update(&self, turn: u32, phase: TurnPhase) -> bool {
        self.turn.current_turn != turn || self.turn.phase != phase
    }

    /// Get formatted turn display text
    pub fn turn_display_text(&self) -> String {
        let phase_text = match self.turn.phase {
            TurnPhase::PlayerTurn => "Player Turn",
            TurnPhase::Processing => "Processing",
            TurnPhase::EnemyTurn => "Enemy Turn",
        };
        format!("Turn: {} - {}", self.turn.current_turn, phase_text)
    }
}

/// System to collect game state and update the centralized UIState resource
pub fn collect_ui_state(
    mut ui_state: ResMut<UIState>,
    turn_counter: Res<TurnCounter>,
    phase: Res<State<TurnPhase>>,
) {
    let current_turn = turn_counter.current;
    let current_phase = *phase.get();

    // Only update if something has changed to avoid unnecessary UI updates
    if ui_state.needs_update(current_turn, current_phase) {
        ui_state.update(current_turn, current_phase);
    }
}

/// Event to notify UI systems that state has been updated
#[derive(Message)]
pub struct UIStateUpdated;

/// System to send UI state update events when state changes
pub fn notify_ui_state_changes(
    ui_state: Res<UIState>,
    mut state_events: MessageWriter<UIStateUpdated>,
) {
    if ui_state.is_changed() && !ui_state.is_added() {
        state_events.write(UIStateUpdated);
    }
}

#[cfg(test)]
mod tests {
    use crate::turn_system::TurnPhase;
    use crate::ui::state::UIState;

    #[test]
    fn test_ui_state_default() {
        let ui_state = UIState::default();
        assert_eq!(ui_state.turn.current_turn, 1);
        assert_eq!(ui_state.turn.phase, TurnPhase::PlayerTurn);
        assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Player Turn");
    }

    #[test]
    fn test_ui_state_update_and_change_detection() {
        let mut ui_state = UIState::default();

        // Initially, UI does not need update for the same turn state
        assert!(!ui_state.needs_update(1, TurnPhase::PlayerTurn));

        // Simulate a phase change that should trigger UI update
        assert!(ui_state.needs_update(1, TurnPhase::Processing));

        // Apply update and verify text
        ui_state.update(1, TurnPhase::Processing);
        assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Processing");
    }
}
