use std::collections::HashMap;

use bevy::prelude::*;

use crate::economy::{Name, NationId, Treasury};
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::logging::TerminalLogEvent;
use crate::ui::menu::AppState;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct DiplomacyPair(NationId, NationId);

impl DiplomacyPair {
    fn new(a: NationId, b: NationId) -> Self {
        if a.0 <= b.0 {
            DiplomacyPair(a, b)
        } else {
            DiplomacyPair(b, a)
        }
    }

    fn contains(&self, nation: NationId) -> bool {
        self.0 == nation || self.1 == nation
    }

    fn other(&self, nation: NationId) -> Option<NationId> {
        if self.0 == nation {
            Some(self.1)
        } else if self.1 == nation {
            Some(self.0)
        } else {
            None
        }
    }
}

/// Relationship tiers used for UI labelling and thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipBand {
    Hostile,
    Unfriendly,
    Neutral,
    Cordial,
    Warm,
    Allied,
}

impl RelationshipBand {
    pub fn label(&self) -> &'static str {
        match self {
            RelationshipBand::Hostile => "Hostile",
            RelationshipBand::Unfriendly => "Unfriendly",
            RelationshipBand::Neutral => "Neutral",
            RelationshipBand::Cordial => "Cordial",
            RelationshipBand::Warm => "Warm",
            RelationshipBand::Allied => "Allied",
        }
    }
}

/// Persistent diplomatic state between two nations.
#[derive(Clone, Debug)]
pub struct DiplomaticRelation {
    pub score: i32,
    pub treaty: TreatyState,
}

impl Default for DiplomaticRelation {
    fn default() -> Self {
        Self {
            score: 0,
            treaty: TreatyState::peace(),
        }
    }
}

impl DiplomaticRelation {
    pub fn band(&self) -> RelationshipBand {
        match self.score {
            ..=-50 => RelationshipBand::Hostile,
            -49..=-11 => RelationshipBand::Unfriendly,
            -10..=10 => RelationshipBand::Neutral,
            11..=39 => RelationshipBand::Cordial,
            40..=69 => RelationshipBand::Warm,
            _ => RelationshipBand::Allied,
        }
    }
}

/// Treaty flags following Imperialism's diplomacy flow.
#[derive(Clone, Debug)]
pub struct TreatyState {
    pub at_war: bool,
    pub consulate: bool,
    pub embassy: bool,
    pub non_aggression_pact: bool,
    pub alliance: bool,
}

impl TreatyState {
    pub fn peace() -> Self {
        Self {
            at_war: false,
            consulate: false,
            embassy: false,
            non_aggression_pact: false,
            alliance: false,
        }
    }
}

/// All relationships between nations.
#[derive(Resource, Default)]
pub struct DiplomacyState {
    relations: HashMap<DiplomacyPair, DiplomaticRelation>,
}

impl DiplomacyState {
    pub fn relation(&self, a: NationId, b: NationId) -> Option<&DiplomaticRelation> {
        self.relations.get(&DiplomacyPair::new(a, b))
    }

    pub fn relation_mut(&mut self, a: NationId, b: NationId) -> &mut DiplomaticRelation {
        let pair = DiplomacyPair::new(a, b);
        self.relations.entry(pair).or_default()
    }

    pub fn ensure_pairs(&mut self, nations: &[NationId]) {
        for (index, &a) in nations.iter().enumerate() {
            for &b in &nations[index + 1..] {
                let pair = DiplomacyPair::new(a, b);
                self.relations.entry(pair).or_default();
            }
        }
    }

    pub fn adjust_score(&mut self, a: NationId, b: NationId, delta: i32) -> i32 {
        let relation = self.relation_mut(a, b);
        relation.score = (relation.score + delta).clamp(-100, 100);
        relation.score
    }

    pub fn set_treaty<F>(&mut self, a: NationId, b: NationId, update: F)
    where
        F: FnOnce(&mut TreatyState),
    {
        let relation = self.relation_mut(a, b);
        update(&mut relation.treaty);
    }

    pub fn relations_for(&self, nation: NationId) -> Vec<(NationId, &DiplomaticRelation)> {
        self.relations
            .iter()
            .filter_map(|(pair, relation)| {
                pair.other(nation)
                    .map(|other| (other, relation))
                    .filter(|_| pair.contains(nation))
            })
            .collect()
    }
}

/// Representation of a recurring aid payment.
#[derive(Clone, Debug)]
pub struct RecurringGrant {
    pub from: NationId,
    pub to: NationId,
    pub amount: i32,
}

#[derive(Resource, Default)]
pub struct ForeignAidLedger {
    recurring: Vec<RecurringGrant>,
}

impl ForeignAidLedger {
    pub fn upsert(&mut self, grant: RecurringGrant) {
        self.recurring
            .retain(|g| !(g.from == grant.from && g.to == grant.to));
        self.recurring.push(grant);
    }

    pub fn cancel(&mut self, from: NationId, to: NationId) -> bool {
        let len_before = self.recurring.len();
        self.recurring.retain(|g| !(g.from == from && g.to == to));
        len_before != self.recurring.len()
    }

    pub fn has_recurring(&self, from: NationId, to: NationId) -> bool {
        self.recurring.iter().any(|g| g.from == from && g.to == to)
    }

    pub fn all(&self) -> &[RecurringGrant] {
        &self.recurring
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct OfferId(u32);

impl OfferId {
    pub fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct DiplomaticOffer {
    pub id: OfferId,
    pub from: NationId,
    pub to: NationId,
    pub kind: DiplomaticOfferKind,
}

impl DiplomaticOffer {
    pub fn new(from: NationId, to: NationId, kind: DiplomaticOfferKind) -> Self {
        Self {
            id: OfferId(0),
            from,
            to,
            kind,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DiplomaticOfferKind {
    OfferPeace,
    Alliance,
    NonAggressionPact,
    ForeignAid { amount: i32, locked: bool },
}

#[derive(Resource, Default)]
pub struct DiplomaticOffers {
    next_id: u32,
    pending: Vec<DiplomaticOffer>,
}

impl DiplomaticOffers {
    pub fn push(&mut self, offer: DiplomaticOffer) {
        let mut offer = offer;
        self.next_id = self.next_id.saturating_add(1);
        offer.id = OfferId(self.next_id);
        self.pending.push(offer);
    }

    pub fn iter_for(&self, nation: NationId) -> impl Iterator<Item = &DiplomaticOffer> {
        self.pending.iter().filter(move |offer| offer.to == nation)
    }

    pub fn has_pending_for(&self, nation: NationId) -> bool {
        self.iter_for(nation).next().is_some()
    }

    pub fn take(&mut self, id: OfferId) -> Option<DiplomaticOffer> {
        if let Some(index) = self.pending.iter().position(|offer| offer.id == id) {
            Some(self.pending.remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

/// Orders issued during the player turn.
#[derive(Message, Debug, Clone)]
pub struct DiplomaticOrder {
    pub actor: NationId,
    pub target: NationId,
    pub kind: DiplomaticOrderKind,
}

#[derive(Debug, Clone)]
pub enum DiplomaticOrderKind {
    DeclareWar,
    OfferPeace,
    EstablishConsulate,
    OpenEmbassy,
    SignNonAggressionPact,
    FormAlliance,
    SendAid { amount: i32, locked: bool },
    CancelAid,
}

/// Tracks UI selection state for diplomacy mode.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DiplomacySelection {
    pub selected: Option<NationId>,
}

pub struct DiplomacyPlugin;

impl Plugin for DiplomacyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiplomacyState>()
            .init_resource::<ForeignAidLedger>()
            .init_resource::<DiplomaticOffers>()
            .init_resource::<DiplomacySelection>()
            .add_message::<DiplomaticOrder>()
            .add_systems(
                Update,
                (
                    sync_diplomatic_pairs,
                    process_diplomatic_orders
                        .run_if(resource_changed::<TurnSystem>)
                        .run_if(|turn: Res<TurnSystem>| turn.phase == TurnPhase::Processing),
                    apply_recurring_aid
                        .run_if(resource_changed::<TurnSystem>)
                        .run_if(|turn: Res<TurnSystem>| turn.phase == TurnPhase::PlayerTurn),
                    decay_relationships
                        .run_if(resource_changed::<TurnSystem>)
                        .run_if(|turn: Res<TurnSystem>| turn.phase == TurnPhase::PlayerTurn),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn sync_diplomatic_pairs(mut state: ResMut<DiplomacyState>, nations: Query<&NationId>) {
    let ids: Vec<NationId> = nations.iter().copied().collect();
    state.ensure_pairs(&ids);
}

fn process_diplomatic_orders(
    mut orders: MessageReader<DiplomaticOrder>,
    mut state: ResMut<DiplomacyState>,
    mut ledger: ResMut<ForeignAidLedger>,
    mut offers: ResMut<DiplomaticOffers>,
    nations: Query<(Entity, &NationId, &Name)>,
    mut treasuries: Query<&mut Treasury>,
    mut log: MessageWriter<TerminalLogEvent>,
) {
    let (id_to_entity, id_to_name) = collect_nation_lookup(&nations);

    for order in orders.read() {
        if order.actor == order.target {
            continue;
        }

        let Some(&actor_entity) = id_to_entity.get(&order.actor) else {
            continue;
        };
        let Some(&target_entity) = id_to_entity.get(&order.target) else {
            continue;
        };

        match &order.kind {
            DiplomaticOrderKind::DeclareWar => {
                let already_at_war = state
                    .relation(order.actor, order.target)
                    .map(|r| r.treaty.at_war)
                    .unwrap_or(false);
                if already_at_war {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} is already at war with {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                state.set_treaty(order.actor, order.target, |t| {
                    t.at_war = true;
                    t.non_aggression_pact = false;
                    t.alliance = false;
                });
                state.adjust_score(order.actor, order.target, -40);
                ledger.cancel(order.actor, order.target);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} has declared war on {}!",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::OfferPeace => {
                let at_war = state
                    .relation(order.actor, order.target)
                    .map(|r| r.treaty.at_war)
                    .unwrap_or(false);
                if !at_war {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} and {} are not currently at war.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                offers.push(DiplomaticOffer::new(
                    order.actor,
                    order.target,
                    DiplomaticOfferKind::OfferPeace,
                ));
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} offered peace to {}.",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::EstablishConsulate => {
                if state
                    .relation(order.actor, order.target)
                    .map(|r| r.treaty.consulate)
                    .unwrap_or(false)
                {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} already maintains a consulate in {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                let relation_score = state
                    .relation(order.actor, order.target)
                    .map(|r| r.score)
                    .unwrap_or_default();
                if relation_score < 0 {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Relations with {} are too poor to open a consulate.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                let afforded = {
                    let mut treasury = match treasuries.get_mut(actor_entity) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if treasury.available() < 500 {
                        log.write(TerminalLogEvent {
                            message: format!(
                                "{} lacks the $500 needed for a consulate in {}.",
                                display_name(&id_to_name, order.actor),
                                display_name(&id_to_name, order.target)
                            ),
                        });
                        false
                    } else {
                        treasury.subtract(500);
                        true
                    }
                };
                if !afforded {
                    continue;
                }

                state.set_treaty(order.actor, order.target, |t| {
                    t.consulate = true;
                });
                state.adjust_score(order.actor, order.target, 5);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} established a consulate in {}.",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::OpenEmbassy => {
                let relation_data = state.relation(order.actor, order.target).cloned();
                let Some(relation) = relation_data else {
                    continue;
                };
                if !relation.treaty.consulate {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "A consulate is required before opening an embassy in {}.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if relation.treaty.embassy {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} already has an embassy in {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if relation.score < 30 {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Relations with {} must be Cordial (30) to open an embassy.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                let afforded = {
                    let mut treasury = match treasuries.get_mut(actor_entity) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if treasury.available() < 5_000 {
                        log.write(TerminalLogEvent {
                            message: format!(
                                "{} lacks the $5,000 needed for an embassy in {}.",
                                display_name(&id_to_name, order.actor),
                                display_name(&id_to_name, order.target)
                            ),
                        });
                        false
                    } else {
                        treasury.subtract(5_000);
                        true
                    }
                };
                if !afforded {
                    continue;
                }

                state.set_treaty(order.actor, order.target, |t| {
                    t.embassy = true;
                });
                state.adjust_score(order.actor, order.target, 10);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} opened an embassy in {}.",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::SignNonAggressionPact => {
                let relation = state.relation(order.actor, order.target).cloned();
                let Some(relation) = relation else { continue };
                if relation.treaty.at_war {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Cannot sign a pact while at war with {}.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if !relation.treaty.embassy {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "An embassy in {} is required before a pact can be signed.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if relation.treaty.non_aggression_pact {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} already has a pact with {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                state.set_treaty(order.actor, order.target, |t| {
                    t.non_aggression_pact = true;
                });
                state.adjust_score(order.actor, order.target, 8);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} signed a non-aggression pact with {}.",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::FormAlliance => {
                let relation = state.relation(order.actor, order.target).cloned();
                let Some(relation) = relation else { continue };
                if relation.treaty.at_war {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Cannot ally while at war with {}.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if !relation.treaty.embassy {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "An embassy in {} is required before an alliance.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if relation.score < 40 {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Relations with {} must be Warm (40) for an alliance.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }
                if relation.treaty.alliance {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} already has an alliance with {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                offers.push(DiplomaticOffer::new(
                    order.actor,
                    order.target,
                    DiplomaticOfferKind::Alliance,
                ));
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} proposed an alliance to {}.",
                        display_name(&id_to_name, order.actor),
                        display_name(&id_to_name, order.target)
                    ),
                });
            }
            DiplomaticOrderKind::SendAid { amount, locked } => {
                if *amount <= 0 {
                    continue;
                }
                let relation = state.relation(order.actor, order.target).cloned();
                let Some(relation) = relation else { continue };
                if relation.treaty.at_war {
                    log.write(TerminalLogEvent {
                        message: format!(
                            "Cannot send aid while at war with {}.",
                            display_name(&id_to_name, order.target)
                        ),
                    });
                    continue;
                }

                let amount = *amount as i64;
                let afforded = {
                    let mut donor_treasury = match treasuries.get_mut(actor_entity) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if donor_treasury.available() < amount {
                        log.write(TerminalLogEvent {
                            message: format!(
                                "{} lacks ${} to fund aid for {}.",
                                display_name(&id_to_name, order.actor),
                                amount,
                                display_name(&id_to_name, order.target)
                            ),
                        });
                        false
                    } else {
                        donor_treasury.subtract(amount);
                        true
                    }
                };
                if !afforded {
                    continue;
                }

                if let Ok(mut receiver_treasury) = treasuries.get_mut(target_entity) {
                    receiver_treasury.add(amount);
                }

                let relation_bonus = (amount / 200).clamp(1, 10) as i32;
                state.adjust_score(order.actor, order.target, relation_bonus);

                if *locked {
                    ledger.upsert(RecurringGrant {
                        from: order.actor,
                        to: order.target,
                        amount: amount as i32,
                    });
                }

                log.write(TerminalLogEvent {
                    message: format!(
                        "{} sent ${} in aid to {}{}.",
                        display_name(&id_to_name, order.actor),
                        amount,
                        display_name(&id_to_name, order.target),
                        if *locked { " (locked grant)" } else { "" }
                    ),
                });
            }
            DiplomaticOrderKind::CancelAid => {
                if ledger.cancel(order.actor, order.target) {
                    state.adjust_score(order.actor, order.target, -5);
                    log.write(TerminalLogEvent {
                        message: format!(
                            "{} cancelled aid to {}.",
                            display_name(&id_to_name, order.actor),
                            display_name(&id_to_name, order.target)
                        ),
                    });
                }
            }
        }
    }
}

pub fn resolve_offer_response(
    offer: DiplomaticOffer,
    accept: bool,
    state: &mut DiplomacyState,
    ledger: &mut ForeignAidLedger,
    nations: &Query<(Entity, &NationId, &Name)>,
    treasuries: &mut Query<&mut Treasury>,
    log: &mut MessageWriter<TerminalLogEvent>,
) {
    let (id_to_entity, id_to_name) = collect_nation_lookup(nations);

    if accept {
        match offer.kind {
            DiplomaticOfferKind::OfferPeace => {
                state.set_treaty(offer.from, offer.to, |t| {
                    t.at_war = false;
                    t.non_aggression_pact = false;
                });
                state.adjust_score(offer.from, offer.to, 15);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} accepted peace with {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::Alliance => {
                state.set_treaty(offer.from, offer.to, |t| {
                    t.alliance = true;
                    t.non_aggression_pact = true;
                });
                state.adjust_score(offer.from, offer.to, 12);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} entered an alliance with {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::NonAggressionPact => {
                state.set_treaty(offer.from, offer.to, |t| {
                    t.non_aggression_pact = true;
                });
                state.adjust_score(offer.from, offer.to, 8);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} accepted a non-aggression pact with {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::ForeignAid { amount, locked } => {
                if let Some(&from_entity) = id_to_entity.get(&offer.from) {
                    if let Ok(mut donor_treasury) = treasuries.get_mut(from_entity) {
                        if donor_treasury.available() < amount as i64 {
                            log.write(TerminalLogEvent {
                                message: format!(
                                    "{} could not afford the ${} aid promised to {}.",
                                    display_name(&id_to_name, offer.from),
                                    amount,
                                    display_name(&id_to_name, offer.to)
                                ),
                            });
                            return;
                        }
                        donor_treasury.subtract(amount as i64);
                    }
                }

                if let Some(&to_entity) = id_to_entity.get(&offer.to) {
                    if let Ok(mut receiver) = treasuries.get_mut(to_entity) {
                        receiver.add(amount as i64);
                    }
                }

                state.adjust_score(offer.from, offer.to, ((amount / 200).max(1)) as i32);

                if locked {
                    ledger.upsert(RecurringGrant {
                        from: offer.from,
                        to: offer.to,
                        amount,
                    });
                }

                log.write(TerminalLogEvent {
                    message: format!(
                        "{} received ${} in aid from {}{}.",
                        display_name(&id_to_name, offer.to),
                        amount,
                        display_name(&id_to_name, offer.from),
                        if locked { " (locked grant)" } else { "" }
                    ),
                });
            }
        }
    } else {
        match offer.kind {
            DiplomaticOfferKind::OfferPeace => {
                state.adjust_score(offer.from, offer.to, -10);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} refused peace with {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::Alliance => {
                state.adjust_score(offer.from, offer.to, -12);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} declined an alliance proposed by {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::NonAggressionPact => {
                state.adjust_score(offer.from, offer.to, -6);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} rejected a non-aggression pact with {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
            DiplomaticOfferKind::ForeignAid { .. } => {
                state.adjust_score(offer.from, offer.to, -3);
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} declined aid from {}.",
                        display_name(&id_to_name, offer.to),
                        display_name(&id_to_name, offer.from)
                    ),
                });
            }
        }
    }
}

fn apply_recurring_aid(
    ledger: Res<ForeignAidLedger>,
    mut state: ResMut<DiplomacyState>,
    nations: Query<(Entity, &NationId, &Name)>,
    mut treasuries: Query<&mut Treasury>,
    mut log: MessageWriter<TerminalLogEvent>,
) {
    let (id_to_entity, id_to_name) = collect_nation_lookup(&nations);

    let grants = ledger.all().to_vec();
    for grant in grants {
        let Some(&from_entity) = id_to_entity.get(&grant.from) else {
            continue;
        };
        let Some(&to_entity) = id_to_entity.get(&grant.to) else {
            continue;
        };

        let amount = grant.amount as i64;
        let afforded = {
            let mut donor_treasury = match treasuries.get_mut(from_entity) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if donor_treasury.available() < amount {
                log.write(TerminalLogEvent {
                    message: format!(
                        "{} could not afford the locked aid payment to {} (missing ${}).",
                        display_name(&id_to_name, grant.from),
                        display_name(&id_to_name, grant.to),
                        amount
                    ),
                });
                false
            } else {
                donor_treasury.subtract(amount);
                true
            }
        };
        if !afforded {
            continue;
        }

        if let Ok(mut receiver) = treasuries.get_mut(to_entity) {
            receiver.add(amount);
        }

        state.adjust_score(grant.from, grant.to, ((amount / 200).max(1)) as i32);

        log.write(TerminalLogEvent {
            message: format!(
                "{} delivered ${} in locked aid to {}.",
                display_name(&id_to_name, grant.from),
                amount,
                display_name(&id_to_name, grant.to)
            ),
        });
    }
}

fn decay_relationships(mut state: ResMut<DiplomacyState>) {
    for relation in state.relations.values_mut() {
        if relation.treaty.at_war {
            continue;
        }
        if relation.score > 0 {
            relation.score -= 1;
        } else if relation.score < 0 {
            relation.score += 1;
        }
    }
}

fn display_name(names: &HashMap<NationId, String>, nation: NationId) -> String {
    names
        .get(&nation)
        .cloned()
        .unwrap_or_else(|| format!("Nation {}", nation.0))
}

fn collect_nation_lookup(
    nations: &Query<(Entity, &NationId, &Name)>,
) -> (HashMap<NationId, Entity>, HashMap<NationId, String>) {
    let mut id_to_entity: HashMap<NationId, Entity> = HashMap::new();
    let mut id_to_name: HashMap<NationId, String> = HashMap::new();
    for (entity, nation_id, name) in nations.iter() {
        id_to_entity.insert(*nation_id, entity);
        id_to_name.insert(*nation_id, name.0.clone());
    }
    (id_to_entity, id_to_name)
}

#[cfg(test)]
mod tests;
