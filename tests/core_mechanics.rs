//! Integration tests for core game mechanics
//!
//! These tests verify basic systems like turn management, UI state, and hex utilities.

#[test]
fn test_turn_counter() {
    use rust_imperialism::turn_system::{TurnCounter, TurnPhase};

    // Test TurnCounter
    let mut counter = TurnCounter::new(1);
    assert_eq!(counter.current, 1);

    counter.increment();
    assert_eq!(counter.current, 2);

    // Test TurnPhase enum
    assert_eq!(TurnPhase::default(), TurnPhase::PlayerTurn);
    assert_ne!(TurnPhase::PlayerTurn, TurnPhase::Processing);
    assert_ne!(TurnPhase::Processing, TurnPhase::EnemyTurn);
}

#[test]
fn test_ui_state() {
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::state::{TurnState, UIState};

    let ui_state = UIState::default();
    // Default display text should reflect default turn state
    assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Player Turn");

    // Test display text generation with a custom turn state
    let ui_state = UIState {
        turn: TurnState {
            current_turn: 5,
            phase: TurnPhase::EnemyTurn,
        },
    };

    assert_eq!(ui_state.turn_display_text(), "Turn: 5 - Enemy Turn");
}

#[test]
fn test_hex_coordinates() {
    use bevy_ecs_tilemap::prelude::TilePos;
    use rust_imperialism::map::TilePosExt;

    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 2, y: 1 };

    let hex1 = pos1.to_hex();
    let hex2 = pos2.to_hex();

    let distance = hex1.distance_to(hex2);
    assert_eq!(distance, 1); // Adjacent tiles should have distance 1
}
