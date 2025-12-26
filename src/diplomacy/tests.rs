use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use moonshine_kind::Instance;

use crate::diplomacy::{
    DiplomacyState, DiplomaticOffer, DiplomaticOfferKind, DiplomaticOffers, DiplomaticOrder,
    DiplomaticOrderKind, ForeignAidLedger, apply_recurring_aid, decay_relationships,
    process_diplomatic_orders, resolve_offer_response, sync_diplomatic_pairs,
};
use crate::economy::{Nation, NationInstance, Treasury};
use crate::turn_system::{TurnPhase, TurnSystem};

fn setup_world() -> World {
    let mut world = World::new();
    world.init_resource::<TurnSystem>();
    world.init_resource::<Messages<DiplomaticOrder>>();
    world.insert_resource(DiplomacyState::default());
    world.insert_resource(ForeignAidLedger::default());
    world.insert_resource(DiplomaticOffers::default());
    world
}

/// Helper to get NationInstance from entity in tests
fn nation_instance(world: &World, entity: Entity) -> NationInstance {
    Instance::<Nation>::from_entity(world.entity(entity))
        .expect("Entity should have Nation component")
}

#[test]
fn consulate_requires_funds_and_relations() {
    let mut world = setup_world();

    let player = world
        .spawn((Nation, Name::new("Player"), Treasury::new(400)))
        .id();
    let minor = world
        .spawn((Nation, Name::new("Minor"), Treasury::new(0)))
        .id();

    let player_inst = nation_instance(&world, player);
    let minor_inst = nation_instance(&world, minor);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;

    // Ensure relations are tracked
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Attempt to open a consulate with insufficient funds (should fail)
    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: player_inst,
            target: minor_inst,
            kind: DiplomaticOrderKind::EstablishConsulate,
        });
    }
    let _ = world.run_system_once(process_diplomatic_orders);

    // Treasury unchanged and no consulate flag set
    let treasury = world.get::<Treasury>(player).unwrap();
    assert_eq!(treasury.total(), 400);
    assert!(
        !world
            .resource::<DiplomacyState>()
            .relation(player_inst, minor_inst)
            .unwrap()
            .treaty
            .consulate
    );

    // Add funds and positive relations then try again
    world.get_mut::<Treasury>(player).unwrap().add(200);
    world
        .resource_mut::<DiplomacyState>()
        .adjust_score(player_inst, minor_inst, 5);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: player_inst,
            target: minor_inst,
            kind: DiplomaticOrderKind::EstablishConsulate,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let treasury = world.get::<Treasury>(player).unwrap();
    assert_eq!(treasury.total(), 100); // 400 + 200 - 500 cost
    assert!(
        world
            .resource::<DiplomacyState>()
            .relation(player_inst, minor_inst)
            .unwrap()
            .treaty
            .consulate
    );
}

#[test]
fn recurring_aid_transfers_each_turn() {
    let mut world = setup_world();

    let donor = world
        .spawn((Nation, Name::new("Donor"), Treasury::new(5_000)))
        .id();
    let recipient = world
        .spawn((Nation, Name::new("Recipient"), Treasury::new(0)))
        .id();

    let donor_inst = nation_instance(&world, donor);
    let recipient_inst = nation_instance(&world, recipient);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;

    // Initialize relations and record the aid order
    let _ = world.run_system_once(sync_diplomatic_pairs);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: donor_inst,
            target: recipient_inst,
            kind: DiplomaticOrderKind::SendAid {
                amount: 1_000,
                locked: true,
            },
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    // At start of next player turn apply recurring aid
    world.resource_mut::<TurnSystem>().phase = TurnPhase::PlayerTurn;
    let _ = world.run_system_once(
        |ledger: Res<ForeignAidLedger>,
         state: ResMut<DiplomacyState>,
         nations: Query<(NationInstance, &Name)>,
         treasuries: Query<&mut Treasury>| {
            apply_recurring_aid(ledger, state, nations, treasuries);
        },
    );

    // Verify funds moved and relation increased
    let donor_treasury = world.get::<Treasury>(donor).unwrap();
    let recipient_treasury = world.get::<Treasury>(recipient).unwrap();
    assert_eq!(donor_treasury.total(), 3_000);
    assert_eq!(recipient_treasury.total(), 2_000);

    let before_score = world
        .resource::<DiplomacyState>()
        .relation(donor_inst, recipient_inst)
        .unwrap()
        .score;
    assert!(before_score >= 5);

    // Decay should not drop wartime relations (already peace)
    let _ = world.run_system_once(decay_relationships);
    let after_score = world
        .resource::<DiplomacyState>()
        .relation(donor_inst, recipient_inst)
        .unwrap()
        .score;
    assert!(after_score <= before_score);
}

#[test]
fn embassy_requires_consulate_and_relations() {
    let mut world = setup_world();

    let empire = world
        .spawn((Nation, Name::new("Empire"), Treasury::new(10_000)))
        .id();
    let neighbor = world
        .spawn((Nation, Name::new("Neighbor"), Treasury::new(0)))
        .id();

    let empire_inst = nation_instance(&world, empire);
    let neighbor_inst = nation_instance(&world, neighbor);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Attempt to open an embassy without a consulate
    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: empire_inst,
            target: neighbor_inst,
            kind: DiplomaticOrderKind::OpenEmbassy,
        });
    }
    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(
        !world
            .resource::<DiplomacyState>()
            .relation(empire_inst, neighbor_inst)
            .unwrap()
            .treaty
            .embassy
    );

    // Grant consulate and relations then open embassy
    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(empire_inst, neighbor_inst, |t| t.consulate = true);
    world
        .resource_mut::<DiplomacyState>()
        .adjust_score(empire_inst, neighbor_inst, 35);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: empire_inst,
            target: neighbor_inst,
            kind: DiplomaticOrderKind::OpenEmbassy,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(
        world
            .resource::<DiplomacyState>()
            .relation(empire_inst, neighbor_inst)
            .unwrap()
            .treaty
            .embassy
    );
}

#[test]
fn declare_war_shifts_world_opinion() {
    let mut world = setup_world();

    let empire = world
        .spawn((Nation, Name::new("Empire"), Treasury::new(1_000)))
        .id();
    let rival = world
        .spawn((Nation, Name::new("Rival"), Treasury::new(1_000)))
        .id();
    let friend = world
        .spawn((Nation, Name::new("Friend"), Treasury::new(1_000)))
        .id();
    let foe = world
        .spawn((Nation, Name::new("Foe"), Treasury::new(1_000)))
        .id();

    let empire_inst = nation_instance(&world, empire);
    let rival_inst = nation_instance(&world, rival);
    let friend_inst = nation_instance(&world, friend);
    let foe_inst = nation_instance(&world, foe);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Friend admires the rival, foe despises them
    {
        let mut state = world.resource_mut::<DiplomacyState>();
        state.adjust_score(friend_inst, rival_inst, 60);
        state.adjust_score(foe_inst, rival_inst, -70);
    }

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: empire_inst,
            target: rival_inst,
            kind: DiplomaticOrderKind::DeclareWar,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let state = world.resource::<DiplomacyState>();
    let relation_with_friend = state
        .relation(empire_inst, friend_inst)
        .expect("friend relation")
        .score;
    let relation_with_foe = state
        .relation(empire_inst, foe_inst)
        .expect("foe relation")
        .score;
    let war_state = state
        .relation(empire_inst, rival_inst)
        .expect("war relation")
        .treaty
        .at_war;

    assert!(war_state, "war flag should be set against rival");
    assert!(
        relation_with_friend < 0,
        "friend of rival should dislike us"
    );
    assert!(relation_with_foe > 0, "enemy of rival should appreciate us");
    assert!(relation_with_friend <= -6);
    assert!(relation_with_foe >= 6);
}

#[test]
fn offer_peace_creates_pending_offer() {
    let mut world = setup_world();

    let player = world
        .spawn((Nation, Name::new("Player"), Treasury::new(1_000)))
        .id();
    let foe = world
        .spawn((Nation, Name::new("Foe"), Treasury::new(1_000)))
        .id();

    let player_inst = nation_instance(&world, player);
    let foe_inst = nation_instance(&world, foe);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(player_inst, foe_inst, |t| {
            t.at_war = true;
        });

    {
        let mut orders = world.resource_mut::<Messages<DiplomaticOrder>>();
        orders.write(DiplomaticOrder {
            actor: player_inst,
            target: foe_inst,
            kind: DiplomaticOrderKind::OfferPeace,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let relation = world
        .resource::<DiplomacyState>()
        .relation(player_inst, foe_inst)
        .unwrap();
    assert!(relation.treaty.at_war);
    assert_eq!(world.resource::<DiplomaticOffers>().len(), 1);
}

#[test]
fn proposing_non_aggression_creates_offer() {
    let mut world = setup_world();

    let player = world
        .spawn((Nation, Name::new("Player"), Treasury::new(2_000)))
        .id();
    let neighbor = world
        .spawn((Nation, Name::new("Neighbor"), Treasury::new(1_000)))
        .id();

    let player_inst = nation_instance(&world, player);
    let neighbor_inst = nation_instance(&world, neighbor);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(player_inst, neighbor_inst, |t| t.embassy = true);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: player_inst,
            target: neighbor_inst,
            kind: DiplomaticOrderKind::SignNonAggressionPact,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let offers = world.resource::<DiplomaticOffers>();
    let mut pending = offers.iter_for(neighbor_inst);
    let offer = pending.next().expect("pact offer present");
    assert!(matches!(offer.kind, DiplomaticOfferKind::NonAggressionPact));
}

#[test]
fn accepting_peace_offer_sets_peace() {
    let mut world = setup_world();

    let player = world
        .spawn((Nation, Name::new("Player"), Treasury::new(1_000)))
        .id();
    let foe = world
        .spawn((Nation, Name::new("Foe"), Treasury::new(1_000)))
        .id();

    let player_inst = nation_instance(&world, player);
    let foe_inst = nation_instance(&world, foe);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(player_inst, foe_inst, |t| {
            t.at_war = true;
        });

    let offer = DiplomaticOffer::new(foe_inst, player_inst, DiplomaticOfferKind::OfferPeace);

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              nations: Query<(NationInstance, &Name)>,
              mut treasuries: Query<&mut Treasury>| {
            resolve_offer_response(
                offer.clone(),
                true,
                &mut state,
                &mut ledger,
                &nations,
                &mut treasuries,
            );
        },
    );

    let relation = world
        .resource::<DiplomacyState>()
        .relation(player_inst, foe_inst)
        .unwrap();
    assert!(!relation.treaty.at_war);
    assert!(relation.score >= 10);
}

#[test]
fn declare_war_triggers_alliance_calls() {
    let mut world = setup_world();

    let attacker = world
        .spawn((Nation, Name::new("Attacker"), Treasury::new(1_000)))
        .id();
    let victim = world
        .spawn((Nation, Name::new("Victim"), Treasury::new(1_000)))
        .id();
    let defender_ally = world
        .spawn((Nation, Name::new("Defender Ally"), Treasury::new(1_000)))
        .id();
    let aggressor_ally = world
        .spawn((Nation, Name::new("Aggressor Ally"), Treasury::new(1_000)))
        .id();

    let attacker_inst = nation_instance(&world, attacker);
    let victim_inst = nation_instance(&world, victim);
    let defender_ally_inst = nation_instance(&world, defender_ally);
    let aggressor_ally_inst = nation_instance(&world, aggressor_ally);

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    {
        let mut state = world.resource_mut::<DiplomacyState>();
        state.set_treaty(victim_inst, defender_ally_inst, |t| {
            t.alliance = true;
            t.non_aggression_pact = true;
            t.embassy = true;
        });
        state.set_treaty(attacker_inst, aggressor_ally_inst, |t| {
            t.alliance = true;
            t.non_aggression_pact = true;
            t.embassy = true;
        });
    }

    {
        let mut orders = world.resource_mut::<Messages<DiplomaticOrder>>();
        orders.write(DiplomaticOrder {
            actor: attacker_inst,
            target: victim_inst,
            kind: DiplomaticOrderKind::DeclareWar,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let offers = world.resource::<DiplomaticOffers>();
    let mut defensive_call = offers.iter_for(defender_ally_inst);
    let defensive = defensive_call.next().expect("defensive call present");
    match defensive.kind {
        DiplomaticOfferKind::JoinWar { enemy, defensive } => {
            assert_eq!(enemy, attacker_inst);
            assert!(defensive);
        }
        _ => panic!("expected defensive join war offer"),
    }

    let mut offensive_call = offers.iter_for(aggressor_ally_inst);
    let offensive = offensive_call.next().expect("offensive call present");
    match offensive.kind {
        DiplomaticOfferKind::JoinWar { enemy, defensive } => {
            assert_eq!(enemy, victim_inst);
            assert!(!defensive);
        }
        _ => panic!("expected offensive join war offer"),
    }
}

#[test]
fn accepting_locked_aid_creates_grant() {
    let mut world = setup_world();

    let donor = world
        .spawn((Nation, Name::new("Donor"), Treasury::new(5_000)))
        .id();
    let recipient = world
        .spawn((Nation, Name::new("Recipient"), Treasury::new(500)))
        .id();

    let donor_inst = nation_instance(&world, donor);
    let recipient_inst = nation_instance(&world, recipient);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(donor_inst, recipient_inst, |t| {
            t.consulate = true;
        });

    let offer = DiplomaticOffer::new(
        donor_inst,
        recipient_inst,
        DiplomaticOfferKind::ForeignAid {
            amount: 1_500,
            locked: true,
        },
    );

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              nations: Query<(NationInstance, &Name)>,
              mut treasuries: Query<&mut Treasury>| {
            resolve_offer_response(
                offer.clone(),
                true,
                &mut state,
                &mut ledger,
                &nations,
                &mut treasuries,
            );
        },
    );

    let donor_treasury = world.get::<Treasury>(donor).unwrap();
    let recipient_treasury = world.get::<Treasury>(recipient).unwrap();
    assert_eq!(donor_treasury.total(), 3_500);
    assert_eq!(recipient_treasury.total(), 2_000);
    assert!(
        world
            .resource::<ForeignAidLedger>()
            .has_recurring(donor_inst, recipient_inst)
    );
}

#[test]
fn accepting_defensive_join_war_sets_war() {
    let mut world = setup_world();

    let aggressor = world
        .spawn((Nation, Name::new("Aggressor"), Treasury::new(1_000)))
        .id();
    let ally = world
        .spawn((Nation, Name::new("Ally"), Treasury::new(1_000)))
        .id();
    let responder = world
        .spawn((Nation, Name::new("Responder"), Treasury::new(1_000)))
        .id();

    let aggressor_inst = nation_instance(&world, aggressor);
    let ally_inst = nation_instance(&world, ally);
    let responder_inst = nation_instance(&world, responder);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(ally_inst, responder_inst, |t| {
            t.alliance = true;
            t.non_aggression_pact = true;
            t.embassy = true;
        });

    let offer = DiplomaticOffer::new(
        ally_inst,
        responder_inst,
        DiplomaticOfferKind::JoinWar {
            enemy: aggressor_inst,
            defensive: true,
        },
    );

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              nations: Query<(NationInstance, &Name)>,
              mut treasuries: Query<&mut Treasury>| {
            resolve_offer_response(
                offer.clone(),
                true,
                &mut state,
                &mut ledger,
                &nations,
                &mut treasuries,
            );
        },
    );

    let state = world.resource::<DiplomacyState>();
    let relation = state
        .relation(responder_inst, aggressor_inst)
        .expect("war relation");
    assert!(relation.treaty.at_war);
    assert!(relation.score <= -34);
}

#[test]
fn declining_defensive_join_war_penalizes() {
    let mut world = setup_world();

    let aggressor = world
        .spawn((Nation, Name::new("Aggressor"), Treasury::new(1_000)))
        .id();
    let attacked_ally = world
        .spawn((Nation, Name::new("Attacked Ally"), Treasury::new(1_000)))
        .id();
    let refuser = world
        .spawn((Nation, Name::new("Refuser"), Treasury::new(1_000)))
        .id();
    let observer = world
        .spawn((Nation, Name::new("Observer"), Treasury::new(1_000)))
        .id();

    let aggressor_inst = nation_instance(&world, aggressor);
    let attacked_ally_inst = nation_instance(&world, attacked_ally);
    let refuser_inst = nation_instance(&world, refuser);
    let observer_inst = nation_instance(&world, observer);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(attacked_ally_inst, refuser_inst, |t| {
            t.alliance = true;
            t.non_aggression_pact = true;
            t.embassy = true;
        });
    world
        .resource_mut::<DiplomacyState>()
        .adjust_score(refuser_inst, observer_inst, 20);

    let offer = DiplomaticOffer::new(
        attacked_ally_inst,
        refuser_inst,
        DiplomaticOfferKind::JoinWar {
            enemy: aggressor_inst,
            defensive: true,
        },
    );

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              nations: Query<(NationInstance, &Name)>,
              mut treasuries: Query<&mut Treasury>| {
            resolve_offer_response(
                offer.clone(),
                false,
                &mut state,
                &mut ledger,
                &nations,
                &mut treasuries,
            );
        },
    );

    let state = world.resource::<DiplomacyState>();
    let alliance_relation = state
        .relation(attacked_ally_inst, refuser_inst)
        .expect("alliance relation");
    assert!(!alliance_relation.treaty.alliance);

    let observer_relation = state
        .relation(refuser_inst, observer_inst)
        .expect("observer relation");
    assert!(observer_relation.score <= 10);
}

#[test]
fn declining_offensive_join_war_preserves_alliance() {
    let mut world = setup_world();

    let aggressor = world
        .spawn((Nation, Name::new("Aggressor"), Treasury::new(1_000)))
        .id();
    let target = world
        .spawn((Nation, Name::new("Target"), Treasury::new(1_000)))
        .id();
    let ally = world
        .spawn((Nation, Name::new("Ally"), Treasury::new(1_000)))
        .id();

    let aggressor_inst = nation_instance(&world, aggressor);
    let target_inst = nation_instance(&world, target);
    let ally_inst = nation_instance(&world, ally);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(aggressor_inst, ally_inst, |t| {
            t.alliance = true;
            t.non_aggression_pact = true;
            t.embassy = true;
        });

    let offer = DiplomaticOffer::new(
        aggressor_inst,
        ally_inst,
        DiplomaticOfferKind::JoinWar {
            enemy: target_inst,
            defensive: false,
        },
    );

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              nations: Query<(NationInstance, &Name)>,
              mut treasuries: Query<&mut Treasury>| {
            resolve_offer_response(
                offer.clone(),
                false,
                &mut state,
                &mut ledger,
                &nations,
                &mut treasuries,
            );
        },
    );

    let state = world.resource::<DiplomacyState>();
    let relation = state
        .relation(aggressor_inst, ally_inst)
        .expect("alliance relation");
    assert!(relation.treaty.alliance);
}
