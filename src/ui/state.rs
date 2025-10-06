use crate::turn_system::{TurnPhase, TurnSystem};
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

impl From<&TurnSystem> for TurnState {
    fn from(turn_system: &TurnSystem) -> Self {
        Self {
            current_turn: turn_system.current_turn,
            phase: turn_system.phase,
        }
    }
}

impl UIState {
    /// Update UI state from world resources
    pub fn update(&mut self, turn_system: &TurnSystem) {
        self.turn = turn_system.into();
    }

    /// Check if any UI-relevant state has changed
    pub fn needs_update(&self, turn_system: &TurnSystem) -> bool {
        self.turn.current_turn != turn_system.current_turn || self.turn.phase != turn_system.phase
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
pub fn collect_ui_state(mut ui_state: ResMut<UIState>, turn_system: Res<TurnSystem>) {
    // Only update if something has changed to avoid unnecessary UI updates
    if ui_state.needs_update(&turn_system) {
        ui_state.update(&turn_system);
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
mod tests;
