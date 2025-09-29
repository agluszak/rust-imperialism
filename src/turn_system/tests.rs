#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::test_with_world;

    test_with_world!(test_turn_system_default, {
        let turn_system = world.resource::<TurnSystem>();
        assert_eq!(turn_system.current_turn, 1);
        assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
        assert!(turn_system.is_player_turn());
    });

    test_with_world!(test_advance_turn_sequence, {
        let mut turn_system = world.resource_mut::<TurnSystem>();

        // PlayerTurn -> Processing
        turn_system.advance_turn();
        assert_eq!(turn_system.phase, TurnPhase::Processing);
        assert_eq!(turn_system.current_turn, 1);

        // Processing -> EnemyTurn
        turn_system.advance_turn();
        assert_eq!(turn_system.phase, TurnPhase::EnemyTurn);
        assert_eq!(turn_system.current_turn, 1);

        // EnemyTurn -> PlayerTurn (new turn)
        turn_system.advance_turn();
        assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
        assert_eq!(turn_system.current_turn, 2);
    });

    test_with_world!(test_end_player_turn, {
        let mut turn_system = world.resource_mut::<TurnSystem>();

        // Can end player turn when it's player's turn
        assert!(turn_system.is_player_turn());
        turn_system.end_player_turn();
        assert_eq!(turn_system.phase, TurnPhase::Processing);

        // Cannot end player turn when it's not player's turn
        turn_system.end_player_turn(); // Should have no effect
        assert_eq!(turn_system.phase, TurnPhase::Processing);
    });

    test_with_world!(test_multiple_turn_cycles, {
        let mut turn_system = world.resource_mut::<TurnSystem>();

        // Complete several full turn cycles
        for expected_turn in 1..=3 {
            assert_eq!(turn_system.current_turn, expected_turn);
            assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);

            // Complete the turn cycle
            turn_system.advance_turn(); // -> Processing
            turn_system.advance_turn(); // -> EnemyTurn
            turn_system.advance_turn(); // -> Next PlayerTurn
        }

        assert_eq!(turn_system.current_turn, 4);
        assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    });

    test_with_world!(test_turn_phase_transitions, {
        let mut turn_system = world.resource_mut::<TurnSystem>();

        let phases = [
            (TurnPhase::PlayerTurn, 1),
            (TurnPhase::Processing, 1),
            (TurnPhase::EnemyTurn, 1),
            (TurnPhase::PlayerTurn, 2),
        ];

        for (expected_phase, expected_turn) in phases {
            assert_eq!(turn_system.phase, expected_phase);
            assert_eq!(turn_system.current_turn, expected_turn);
            turn_system.advance_turn();
        }
    });

    #[test]
    fn test_turn_phase_equality() {
        assert_eq!(TurnPhase::PlayerTurn, TurnPhase::PlayerTurn);
        assert_ne!(TurnPhase::PlayerTurn, TurnPhase::Processing);
        assert_ne!(TurnPhase::Processing, TurnPhase::EnemyTurn);
    }

    #[test]
    fn test_turn_phase_clone() {
        let phase = TurnPhase::PlayerTurn;
        let cloned = phase.clone();
        assert_eq!(phase, cloned);
    }

    #[test]
    fn test_turn_phase_copy() {
        let phase = TurnPhase::EnemyTurn;
        let copied = phase; // Should work because it implements Copy
        assert_eq!(phase, copied);
    }
}
