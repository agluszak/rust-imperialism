//! Integration tests for rust-imperialism game systems
//!
//! These tests demonstrate ECS testing patterns and verify core game mechanics

/// Test turn system functionality
#[test]
fn test_turn_system() {
    use rust_imperialism::turn_system::{TurnPhase, TurnSystem};

    let mut turn_system = TurnSystem::default();
    assert_eq!(turn_system.current_turn, 1);
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::Processing);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::EnemyTurn);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    assert_eq!(turn_system.current_turn, 2); // New turn
}

/// Test UI state management (simplified)
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

/// Test hex coordinate utilities
#[test]
fn test_hex_coordinates() {
    use bevy_ecs_tilemap::prelude::TilePos;
    use rust_imperialism::tile_pos::TilePosExt;

    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 2, y: 1 };

    let hex1 = pos1.to_hex();
    let hex2 = pos2.to_hex();

    let distance = hex1.distance_to(hex2);
    assert_eq!(distance, 1); // Adjacent tiles should have distance 1
}
