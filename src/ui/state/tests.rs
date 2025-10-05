#![cfg(test)]

use super::*;
use crate::turn_system::{TurnPhase, TurnSystem};

#[test]
fn test_ui_state_default() {
    let ui_state = UIState::default();
    assert_eq!(ui_state.turn.current_turn, 1);
    assert_eq!(ui_state.turn.phase, TurnPhase::PlayerTurn);
    assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Player Turn");
}

#[test]
fn test_turn_state_conversion() {
    let mut turn_system = TurnSystem::default();
    turn_system.advance_turn(); // Move to Processing

    let turn_state = TurnState::from(&turn_system);
    assert_eq!(turn_state.current_turn, 1);
    assert_eq!(turn_state.phase, TurnPhase::Processing);
}

#[test]
fn test_ui_state_update_and_change_detection() {
    let mut ui_state = UIState::default();
    let mut turn_system = TurnSystem::default();

    // Initially, UI does not need update for the same turn state
    assert!(!ui_state.needs_update(&turn_system));

    // Simulate a phase advancement that should trigger UI update
    turn_system.advance_turn(); // PlayerTurn -> Processing
    assert!(ui_state.needs_update(&turn_system));

    // Apply update and verify text
    ui_state.update(&turn_system);
    assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Processing");
}
