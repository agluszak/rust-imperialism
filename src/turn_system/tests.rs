use bevy::prelude::*;

use crate::turn_system::{TurnCounter, TurnPhase, TurnSystem};

#[test]
fn test_turn_counter_default() {
    let counter = TurnCounter::default();
    assert_eq!(counter.current, 0);
}

#[test]
fn test_turn_counter_new() {
    let counter = TurnCounter::new(5);
    assert_eq!(counter.current, 5);
}

#[test]
fn test_turn_counter_increment() {
    let mut counter = TurnCounter::new(1);
    counter.increment();
    assert_eq!(counter.current, 2);
    counter.increment();
    assert_eq!(counter.current, 3);
}

#[test]
fn test_turn_phase_default() {
    let phase = TurnPhase::default();
    assert_eq!(phase, TurnPhase::PlayerTurn);
}

#[test]
fn test_turn_phase_equality() {
    assert_eq!(TurnPhase::PlayerTurn, TurnPhase::PlayerTurn);
    assert_ne!(TurnPhase::PlayerTurn, TurnPhase::Processing);
    assert_ne!(TurnPhase::Processing, TurnPhase::EnemyTurn);
}

#[test]
fn test_turn_phase_clone() {
    let phase = TurnPhase::PlayerTurn;
    let cloned = phase;
    assert_eq!(phase, cloned);
}

#[test]
fn test_turn_phase_copy() {
    let phase = TurnPhase::EnemyTurn;
    let copied = phase;
    assert_eq!(phase, copied);
}

#[test]
fn test_legacy_turn_system_default() {
    let turn_system = TurnSystem::default();
    assert_eq!(turn_system.current_turn, 1);
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    assert!(turn_system.is_player_turn());
}

#[test]
fn test_legacy_turn_system_is_player_turn() {
    let mut turn_system = TurnSystem::default();
    assert!(turn_system.is_player_turn());

    turn_system.phase = TurnPhase::Processing;
    assert!(!turn_system.is_player_turn());

    turn_system.phase = TurnPhase::EnemyTurn;
    assert!(!turn_system.is_player_turn());
}
