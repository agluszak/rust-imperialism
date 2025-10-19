use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::{
    DiplomacyState, DiplomaticOffer, DiplomaticOfferKind, DiplomaticOffers, DiplomaticOrder,
    DiplomaticOrderKind, ForeignAidLedger, TradePolicy, TradePolicyLedger, apply_recurring_aid,
    decay_relationships, process_diplomatic_orders, resolve_offer_response, sync_diplomatic_pairs,
};
use crate::economy::{Name, NationId, Treasury};
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::logging::TerminalLogEvent;

fn setup_world() -> World {
    let mut world = World::new();
    world.init_resource::<TurnSystem>();
    world.init_resource::<Messages<TerminalLogEvent>>();
    world.init_resource::<Messages<DiplomaticOrder>>();
    world.insert_resource(DiplomacyState::default());
    world.insert_resource(ForeignAidLedger::default());
    world.insert_resource(DiplomaticOffers::default());
    world.insert_resource(TradePolicyLedger::default());
    world
}

#[test]
fn consulate_requires_funds_and_relations() {
    let mut world = setup_world();

    let player = world
        .spawn((NationId(1), Name("Player".into()), Treasury::new(400)))
        .id();
    let _minor = world
        .spawn((NationId(2), Name("Minor".into()), Treasury::new(0)))
        .id();

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;

    // Ensure relations are tracked
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Attempt to open a consulate with insufficient funds (should fail)
    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
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
            .relation(NationId(1), NationId(2))
            .unwrap()
            .treaty
            .consulate
    );

    // Add funds and positive relations then try again
    world.get_mut::<Treasury>(player).unwrap().add(200);
    world
        .resource_mut::<DiplomacyState>()
        .adjust_score(NationId(1), NationId(2), 5);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::EstablishConsulate,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let treasury = world.get::<Treasury>(player).unwrap();
    assert_eq!(treasury.total(), 100); // 400 + 200 - 500 cost
    assert!(
        world
            .resource::<DiplomacyState>()
            .relation(NationId(1), NationId(2))
            .unwrap()
            .treaty
            .consulate
    );
}

#[test]
fn recurring_aid_transfers_each_turn() {
    let mut world = setup_world();

    let donor = world
        .spawn((NationId(1), Name("Donor".into()), Treasury::new(5_000)))
        .id();
    let recipient = world
        .spawn((NationId(2), Name("Recipient".into()), Treasury::new(0)))
        .id();

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;

    // Initialize relations and record the aid order
    let _ = world.run_system_once(sync_diplomatic_pairs);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
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
         nations: Query<(Entity, &NationId, &Name)>,
         treasuries: Query<&mut Treasury>,
         log: MessageWriter<TerminalLogEvent>| {
            apply_recurring_aid(ledger, state, nations, treasuries, log);
        },
    );

    // Verify funds moved and relation increased
    let donor_treasury = world.get::<Treasury>(donor).unwrap();
    let recipient_treasury = world.get::<Treasury>(recipient).unwrap();
    assert_eq!(donor_treasury.total(), 3_000);
    assert_eq!(recipient_treasury.total(), 2_000);

    let before_score = world
        .resource::<DiplomacyState>()
        .relation(NationId(1), NationId(2))
        .unwrap()
        .score;
    assert!(before_score >= 5);

    // Decay should not drop wartime relations (already peace)
    let _ = world.run_system_once(decay_relationships);
    let after_score = world
        .resource::<DiplomacyState>()
        .relation(NationId(1), NationId(2))
        .unwrap()
        .score;
    assert!(after_score <= before_score);
}

#[test]
fn embassy_requires_consulate_and_relations() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Empire".into()), Treasury::new(10_000)));
    world.spawn((NationId(2), Name("Neighbor".into()), Treasury::new(0)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Attempt to open an embassy without a consulate
    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::OpenEmbassy,
        });
    }
    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(
        !world
            .resource::<DiplomacyState>()
            .relation(NationId(1), NationId(2))
            .unwrap()
            .treaty
            .embassy
    );

    // Grant consulate and relations then open embassy
    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| t.consulate = true);
    world
        .resource_mut::<DiplomacyState>()
        .adjust_score(NationId(1), NationId(2), 35);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::OpenEmbassy,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(
        world
            .resource::<DiplomacyState>()
            .relation(NationId(1), NationId(2))
            .unwrap()
            .treaty
            .embassy
    );
}

#[test]
fn declare_war_shifts_world_opinion() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Empire".into()), Treasury::new(1_000)));
    world.spawn((NationId(2), Name("Rival".into()), Treasury::new(1_000)));
    world.spawn((NationId(3), Name("Friend".into()), Treasury::new(1_000)));
    world.spawn((NationId(4), Name("Foe".into()), Treasury::new(1_000)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    // Friend admires the rival, foe despises them
    {
        let mut state = world.resource_mut::<DiplomacyState>();
        state.adjust_score(NationId(3), NationId(2), 60);
        state.adjust_score(NationId(4), NationId(2), -70);
    }

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::DeclareWar,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let state = world.resource::<DiplomacyState>();
    let relation_with_friend = state
        .relation(NationId(1), NationId(3))
        .expect("friend relation")
        .score;
    let relation_with_foe = state
        .relation(NationId(1), NationId(4))
        .expect("foe relation")
        .score;
    let war_state = state
        .relation(NationId(1), NationId(2))
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

    world.spawn((NationId(1), Name("Player".into()), Treasury::new(1_000)));
    world.spawn((NationId(2), Name("Foe".into()), Treasury::new(1_000)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.at_war = true;
        });

    {
        let mut orders = world.resource_mut::<Messages<DiplomaticOrder>>();
        orders.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::OfferPeace,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let relation = world
        .resource::<DiplomacyState>()
        .relation(NationId(1), NationId(2))
        .unwrap();
    assert!(relation.treaty.at_war);
    assert_eq!(world.resource::<DiplomaticOffers>().len(), 1);
}

#[test]
fn accepting_peace_offer_sets_peace() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Player".into()), Treasury::new(1_000)));
    world.spawn((NationId(2), Name("Foe".into()), Treasury::new(1_000)));

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.at_war = true;
        });

    let offer = DiplomaticOffer::new(NationId(2), NationId(1), DiplomaticOfferKind::OfferPeace);

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              mut trade_policies: ResMut<TradePolicyLedger>,
              nations: Query<(Entity, &NationId, &Name)>,
              mut treasuries: Query<&mut Treasury>,
              mut log: MessageWriter<TerminalLogEvent>| {
            resolve_offer_response(
                offer.clone(),
                true,
                &mut state,
                &mut ledger,
                &mut trade_policies,
                &nations,
                &mut treasuries,
                &mut log,
            );
        },
    );

    let relation = world
        .resource::<DiplomacyState>()
        .relation(NationId(1), NationId(2))
        .unwrap();
    assert!(!relation.treaty.at_war);
    assert!(relation.score >= 10);
}

#[test]
fn accepting_locked_aid_creates_grant() {
    let mut world = setup_world();

    let donor = world
        .spawn((NationId(1), Name("Donor".into()), Treasury::new(5_000)))
        .id();
    let recipient = world
        .spawn((NationId(2), Name("Recipient".into()), Treasury::new(500)))
        .id();

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.consulate = true;
        });

    let offer = DiplomaticOffer::new(
        NationId(1),
        NationId(2),
        DiplomaticOfferKind::ForeignAid {
            amount: 1_500,
            locked: true,
        },
    );

    let _ = world.run_system_once(
        move |mut state: ResMut<DiplomacyState>,
              mut ledger: ResMut<ForeignAidLedger>,
              mut trade_policies: ResMut<TradePolicyLedger>,
              nations: Query<(Entity, &NationId, &Name)>,
              mut treasuries: Query<&mut Treasury>,
              mut log: MessageWriter<TerminalLogEvent>| {
            resolve_offer_response(
                offer.clone(),
                true,
                &mut state,
                &mut ledger,
                &mut trade_policies,
                &nations,
                &mut treasuries,
                &mut log,
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
            .has_recurring(NationId(1), NationId(2))
    );
}

#[test]
fn trade_subsidy_requires_consulate() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Empire".into()), Treasury::new(10_000)));
    world.spawn((NationId(2), Name("Partner".into()), Treasury::new(0)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::SetTradeSubsidy { percent: 5 },
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(matches!(
        world
            .resource::<TradePolicyLedger>()
            .policy(NationId(1), NationId(2)),
        TradePolicy::Neutral
    ));

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.consulate = true;
        });

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::SetTradeSubsidy { percent: 5 },
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(matches!(
        world
            .resource::<TradePolicyLedger>()
            .policy(NationId(1), NationId(2)),
        TradePolicy::Subsidy(percent) if percent == 5
    ));
}

#[test]
fn declare_war_imposes_boycott_policy() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Empire".into()), Treasury::new(1_000)));
    world.spawn((NationId(2), Name("Rival".into()), Treasury::new(1_000)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::DeclareWar,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    let ledger = world.resource::<TradePolicyLedger>();
    assert!(matches!(
        ledger.policy(NationId(1), NationId(2)),
        TradePolicy::Boycott
    ));
    assert!(matches!(
        ledger.policy(NationId(2), NationId(1)),
        TradePolicy::Boycott
    ));
}

#[test]
fn lifting_boycott_requires_peace() {
    let mut world = setup_world();

    world.spawn((NationId(1), Name("Empire".into()), Treasury::new(5_000)));
    world.spawn((NationId(2), Name("Neighbor".into()), Treasury::new(5_000)));

    world.resource_mut::<TurnSystem>().phase = TurnPhase::Processing;
    let _ = world.run_system_once(sync_diplomatic_pairs);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.consulate = true;
        });

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::SetTradeBoycott,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.at_war = true;
        });

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::LiftTradeBoycott,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(matches!(
        world
            .resource::<TradePolicyLedger>()
            .policy(NationId(1), NationId(2)),
        TradePolicy::Boycott
    ));

    world
        .resource_mut::<DiplomacyState>()
        .set_treaty(NationId(1), NationId(2), |t| {
            t.at_war = false;
        });

    {
        let mut messages = world.resource_mut::<Messages<DiplomaticOrder>>();
        messages.write(DiplomaticOrder {
            actor: NationId(1),
            target: NationId(2),
            kind: DiplomaticOrderKind::LiftTradeBoycott,
        });
    }

    let _ = world.run_system_once(process_diplomatic_orders);

    assert!(matches!(
        world
            .resource::<TradePolicyLedger>()
            .policy(NationId(1), NationId(2)),
        TradePolicy::Neutral
    ));
}
