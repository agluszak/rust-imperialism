use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::handle_turn_input;
use crate::diplomacy::{DiplomaticOffer, DiplomaticOfferKind, DiplomaticOffers};
use crate::economy::{Name, NationId, PlayerNation, Treasury};
use crate::test_utils::*;
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::logging::TerminalLogEvent;

#[test]
fn test_turn_system_default() {
    let world = create_test_world();
    let turn_system = world.resource::<TurnSystem>();
    assert_eq!(turn_system.current_turn, 1);
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    assert!(turn_system.is_player_turn());
}

#[test]
fn test_advance_turn_sequence() {
    let mut world = create_test_world();
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
}

#[test]
fn test_end_player_turn() {
    let mut world = create_test_world();
    let mut turn_system = world.resource_mut::<TurnSystem>();

    // Can end player turn when it's player's turn
    assert!(turn_system.is_player_turn());
    turn_system.end_player_turn();
    assert_eq!(turn_system.phase, TurnPhase::Processing);

    // Cannot end player turn when it's not player's turn
    turn_system.end_player_turn(); // Should have no effect
    assert_eq!(turn_system.phase, TurnPhase::Processing);
}

#[test]
fn test_multiple_turn_cycles() {
    let mut world = create_test_world();
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
}

#[test]
fn test_turn_phase_transitions() {
    let mut world = create_test_world();
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
    let copied = phase; // Should work because it implements Copy
    assert_eq!(phase, copied);
}

#[test]
fn pending_offers_block_turn_end() {
    let mut world = create_test_world();
    world.init_resource::<Messages<TerminalLogEvent>>();
    world.insert_resource(DiplomaticOffers::default());

    let player_entity = world
        .spawn((NationId(1), Name("Player".into()), Treasury::new(1_000)))
        .id();
    world.flush();
    let player_nation =
        PlayerNation::from_entity(&world, player_entity).expect("Failed to create PlayerNation");
    world.insert_resource(player_nation);
    world.spawn((NationId(2), Name("Foe".into()), Treasury::new(1_000)));

    world
        .resource_mut::<DiplomaticOffers>()
        .push(DiplomaticOffer::new(
            NationId(2),
            NationId(1),
            DiplomaticOfferKind::OfferPeace,
        ));

    world.insert_resource(ButtonInput::<KeyCode>::default());
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);

    let _ = world.run_system_once(
        |keys: Res<ButtonInput<KeyCode>>,
         turn_system: ResMut<TurnSystem>,
         log: MessageWriter<TerminalLogEvent>,
         offers: Res<DiplomaticOffers>,
         player: Res<PlayerNation>,
         nation_ids: Query<&NationId>| {
            handle_turn_input(
                keys,
                turn_system,
                log,
                Some(offers),
                Some(player),
                nation_ids,
            );
        },
    );

    assert_eq!(world.resource::<TurnSystem>().phase, TurnPhase::PlayerTurn);
}
