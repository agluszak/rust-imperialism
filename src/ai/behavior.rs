#![allow(clippy::type_complexity)]

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};
use big_brain::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::ai::context::{
    AiPlanLedger, MacroTag, MarketView, TransportAnalysis, TurnCandidates, enemy_turn_entered,
    gather_turn_candidates, resource_target_days, update_belief_state_system,
    update_market_view_system, update_transport_analysis_system,
};
use crate::ai::markers::{AiControlledCivilian, AiNation};
use crate::ai::trade::build_market_buy_order;
use crate::civilians::order_validation::tile_owned_by_nation;
use crate::civilians::types::{
    Civilian, CivilianKind, CivilianOrder, CivilianOrderKind, ProspectingKnowledge,
};
use crate::economy::goods::Good;
use crate::economy::nation::{Capital, NationHandle};
use crate::economy::stockpile::Stockpile;
use crate::economy::transport::{ImprovementKind, PlaceImprovement, Rails, ordered_edge};
use crate::map::province::{Province, TileProvince};
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::messages::civilians::CivilianCommand;
use crate::orders::{OrdersOut, flush_orders_to_queue};
use crate::resources::{DevelopmentLevel, TileResource};
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

pub(crate) const RNG_BASE_SEED: u64 = 0xA1_51_23_45;

/// Registers Big Brain and the systems that drive simple AI-controlled civilians.
pub struct AiBehaviorPlugin;

#[derive(Component, Debug, Default)]
struct AiOrderCache {
    improvement: Option<CivilianOrderKind>,
    rail: Option<CivilianOrderKind>,
    movement: Option<CivilianOrderKind>,
}

const INVESTMENT_COOLDOWN_TURNS: u8 = 4;
const RAIL_COOLDOWN_TURNS: u8 = 3;

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct ResourceShortage {
    pub good: Good,
    pub target_days: f32,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiSet {
    Analysis,
    EmitOrders,
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct Bottleneck {
    pub from: TilePos,
    pub to: TilePos,
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct InvestMinorScore {
    pub minor: crate::ai::context::MinorId,
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct BuyResource {
    pub good: Good,
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct UpgradeRail {
    pub from: TilePos,
    pub to: TilePos,
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct InvestInMinor {
    pub minor: crate::ai::context::MinorId,
}

impl Plugin for AiBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiRng>()
            .init_resource::<OrdersOut>()
            .add_plugins(BigBrainPlugin::new(PreUpdate))
            .configure_sets(
                PreUpdate,
                (
                    AiSet::Analysis,
                    BigBrainSet::Scorers,
                    BigBrainSet::Actions,
                    AiSet::EmitOrders,
                )
                    .chain(),
            )
            .add_systems(
                PreUpdate,
                gate_ai_turn
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                (
                    update_belief_state_system,
                    update_market_view_system,
                    update_transport_analysis_system,
                    gather_turn_candidates,
                    rebuild_thinker_if_needed,
                )
                    .in_set(AiSet::Analysis)
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameMode::Map))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                (
                    resource_shortage_scorer,
                    bottleneck_scorer,
                    invest_in_minor_scorer,
                )
                    .in_set(BigBrainSet::Scorers)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                (
                    buy_resource_action,
                    upgrade_rail_action,
                    invest_in_minor_action,
                )
                    .in_set(BigBrainSet::Actions)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                flush_orders_to_queue
                    .in_set(AiSet::EmitOrders)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                (
                    tag_ai_owned_civilians,
                    untag_non_ai_owned_civilians,
                    initialize_ai_thinkers,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                PreUpdate,
                (
                    reset_ai_rng_on_enemy_turn,
                    reset_ai_civilian_actions,
                    reset_ai_action_states,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_entered),
            )
            .add_systems(
                PreUpdate,
                (
                    ready_to_act_scorer,
                    has_rail_target_scorer,
                    has_improvement_target_scorer,
                    has_move_target_scorer,
                )
                    .in_set(BigBrainSet::Scorers)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                PreUpdate,
                (
                    build_rail_action,
                    build_improvement_action,
                    move_to_target_action,
                    skip_turn_action,
                )
                    .in_set(BigBrainSet::Actions)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            );
    }
}

/// Simple deterministic RNG used by the AI so behavior is replayable.
#[derive(Resource)]
pub struct AiRng(StdRng);

impl Default for AiRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(RNG_BASE_SEED))
    }
}

fn tag_ai_owned_civilians(
    mut commands: Commands,
    ai_nations: Query<(), With<AiNation>>,
    untagged: Query<(Entity, &Civilian), Without<AiControlledCivilian>>,
) {
    for (entity, civilian) in &untagged {
        if ai_nations.get(civilian.owner).is_ok() {
            commands.entity(entity).insert(AiControlledCivilian);
        }
    }
}

fn untag_non_ai_owned_civilians(
    mut commands: Commands,
    ai_nations: Query<(), With<AiNation>>,
    tagged: Query<(Entity, &Civilian), With<AiControlledCivilian>>,
) {
    for (entity, civilian) in &tagged {
        if ai_nations.get(civilian.owner).is_err() {
            commands
                .entity(entity)
                .remove::<AiControlledCivilian>()
                .remove::<AiOrderCache>()
                .remove::<Thinker>()
                .remove::<ReadyToAct>()
                .remove::<HasRailTarget>()
                .remove::<HasImprovementTarget>()
                .remove::<HasMoveTarget>()
                .remove::<BuildRailOrder>()
                .remove::<BuildImprovementOrder>()
                .remove::<MoveTowardsOwnedTile>()
                .remove::<SkipTurnOrder>();
        }
    }
}

/// Scorer that determines whether a civilian is ready to issue a new order.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct ReadyToAct;

/// Checks whether an engineer should extend the rail network.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct HasRailTarget;

/// Issues a rail construction order gathered during scoring.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct BuildRailOrder;

/// Checks whether a civilian has a nearby tile they can improve.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct HasImprovementTarget;

/// Issues an improvement order gathered during scoring.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct BuildImprovementOrder;

/// Checks whether a civilian should move to search for opportunities.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct HasMoveTarget;

/// Issues a move order toward a nearby owned tile.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct MoveTowardsOwnedTile;

/// Fallback action when no better work is available.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct SkipTurnOrder;

fn rebuild_thinker_if_needed(
    mut commands: Commands,
    ai_query: Query<(Entity, Option<&Thinker>), With<AiNation>>,
    candidates: Res<TurnCandidates>,
) {
    for (entity, thinker) in ai_query.iter() {
        let mut builder = Thinker::build().label("ai_macro").picker(Highest);
        let mut attached = false;

        for candidate in candidates.for_actor(entity) {
            match &candidate.tag {
                MacroTag::BuyResource { good } => {
                    builder = builder.when(
                        ResourceShortage {
                            good: *good,
                            target_days: resource_target_days(*good),
                        },
                        BuyResource { good: *good },
                    );
                }
                MacroTag::UpgradeRail { from, to } => {
                    builder = builder.when(
                        Bottleneck {
                            from: *from,
                            to: *to,
                        },
                        UpgradeRail {
                            from: *from,
                            to: *to,
                        },
                    );
                }
                MacroTag::InvestMinor { minor } => {
                    builder = builder.when(
                        InvestMinorScore { minor: *minor },
                        InvestInMinor { minor: *minor },
                    );
                }
            }
            attached = true;
        }

        if attached {
            commands.entity(entity).insert(builder);
        } else if thinker.is_some() {
            commands.entity(entity).remove::<Thinker>();
        }
    }
}

fn resource_shortage_scorer(
    stockpiles: Query<&Stockpile>,
    mut scorers: Query<(&Actor, &ResourceShortage, &mut Score)>,
) {
    for (Actor(actor), shortage, mut score) in &mut scorers {
        let Ok(stockpile) = stockpiles.get(*actor) else {
            score.set(0.0);
            continue;
        };

        let available = stockpile.get_available(shortage.good) as f32;
        let target = shortage.target_days.max(1.0);
        let urgency = ((target - available) / target).clamp(0.0, 1.0);
        score.set(urgency);
    }
}

fn bottleneck_scorer(
    transport: Res<TransportAnalysis>,
    mut scorers: Query<(&Actor, &Bottleneck, &mut Score)>,
) {
    for (Actor(actor), bottleneck, mut score) in &mut scorers {
        let Some(candidate) = transport
            .candidates_for(*actor)
            .iter()
            .find(|entry| entry.from == bottleneck.from && entry.to == bottleneck.to)
        else {
            score.set(0.0);
            continue;
        };
        score.set(candidate.marginal_gain.clamp(0.0, 1.0));
    }
}

fn invest_in_minor_scorer(mut scorers: Query<&mut Score, With<InvestMinorScore>>) {
    for mut score in &mut scorers {
        score.set(0.2);
    }
}

fn buy_resource_action(
    mut actions: Query<(&Actor, &BuyResource, &mut ActionState)>,
    stockpiles: Query<&Stockpile>,
    handles: Query<&NationHandle>,
    market: Res<MarketView>,
    mut orders: ResMut<OrdersOut>,
    mut ledger: ResMut<AiPlanLedger>,
) {
    for (Actor(actor), buy, mut state) in &mut actions {
        match *state {
            ActionState::Requested => {
                let Ok(stockpile) = stockpiles.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };
                let Ok(handle) = handles.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                let available = stockpile.get_available(buy.good);
                let desired = resource_target_days(buy.good).round() as u32;
                let Some(qty) = market.recommended_buy_qty(buy.good, available, desired) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if qty == 0 {
                    *state = ActionState::Failure;
                    continue;
                }

                orders.queue_market(build_market_buy_order(handle, buy.good, qty));
                ledger.apply_cooldown(*actor, MacroTag::BuyResource { good: buy.good }, 1);
                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn upgrade_rail_action(
    mut actions: Query<(&Actor, &UpgradeRail, &mut ActionState)>,
    mut orders: ResMut<OrdersOut>,
    mut ledger: ResMut<AiPlanLedger>,
) {
    for (Actor(actor), upgrade, mut state) in &mut actions {
        match *state {
            ActionState::Requested => {
                orders.queue_transport(PlaceImprovement {
                    a: upgrade.from,
                    b: upgrade.to,
                    kind: ImprovementKind::Rail,
                    engineer: None,
                });
                ledger.apply_cooldown(
                    *actor,
                    MacroTag::UpgradeRail {
                        from: upgrade.from,
                        to: upgrade.to,
                    },
                    RAIL_COOLDOWN_TURNS,
                );
                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn invest_in_minor_action(
    mut actions: Query<(&Actor, &InvestInMinor, &mut ActionState)>,
    mut ledger: ResMut<AiPlanLedger>,
) {
    for (Actor(actor), invest, mut state) in &mut actions {
        match *state {
            ActionState::Requested => {
                ledger.apply_cooldown(
                    *actor,
                    MacroTag::InvestMinor {
                        minor: invest.minor,
                    },
                    INVESTMENT_COOLDOWN_TURNS,
                );
                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn gate_ai_turn(
    turn: Res<TurnSystem>,
    mut ledger: ResMut<AiPlanLedger>,
    mut orders: ResMut<OrdersOut>,
    mut candidates: ResMut<TurnCandidates>,
    mut last_turn: Local<Option<u32>>,
) {
    if Some(turn.current_turn) != *last_turn {
        ledger.advance_turn(turn.current_turn);
        orders.clear();
        candidates.clear();
        *last_turn = Some(turn.current_turn);
    }
}

fn enemy_turn_active(turn: Res<TurnSystem>) -> bool {
    turn.phase == TurnPhase::EnemyTurn
}

fn reset_ai_rng_on_enemy_turn(mut rng: ResMut<AiRng>, turn: Res<TurnSystem>) {
    rng.0 = StdRng::seed_from_u64(RNG_BASE_SEED ^ u64::from(turn.current_turn));
}

/// Reset has_moved for AI-controlled civilians at the start of enemy turn.
fn reset_ai_civilian_actions(mut civilians: Query<&mut Civilian, With<AiControlledCivilian>>) {
    for mut civilian in civilians.iter_mut() {
        civilian.has_moved = false;
    }
}

/// Reset all AI action states to Init at the start of each enemy turn.
/// Big-brain actions stay in Success/Failure state unless explicitly reset.
fn reset_ai_action_states(
    mut actions: Query<
        &mut ActionState,
        (
            With<Actor>,
            Or<(
                With<BuildRailOrder>,
                With<BuildImprovementOrder>,
                With<MoveTowardsOwnedTile>,
                With<SkipTurnOrder>,
                With<BuyResource>,
                With<UpgradeRail>,
                With<InvestInMinor>,
                With<crate::ai::trade::PlanBuildingFocus>,
                With<crate::ai::trade::ApplyProductionPlan>,
                With<crate::ai::trade::PlanMarketOrders>,
                With<crate::ai::trade::EconomyIdleAction>,
            )>,
        ),
    >,
) {
    for mut state in actions.iter_mut() {
        if *state == ActionState::Success || *state == ActionState::Failure {
            *state = ActionState::Init;
        }
    }
}

fn initialize_ai_thinkers(
    mut commands: Commands,
    new_units: Query<Entity, (With<AiControlledCivilian>, Without<AiOrderCache>)>,
) {
    for entity in &new_units {
        commands.entity(entity).insert((
            AiOrderCache::default(),
            Thinker::build()
                .label("ai_civilian")
                .picker(FirstToScore { threshold: 0.5 })
                .when(HasRailTarget, BuildRailOrder)
                .when(HasImprovementTarget, BuildImprovementOrder)
                .when(HasMoveTarget, MoveTowardsOwnedTile)
                .when(ReadyToAct, SkipTurnOrder),
        ));
    }
}

fn ready_to_act_scorer(
    civilians: Query<&Civilian>,
    mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<ReadyToAct>>,
) {
    for (Actor(actor), mut score, span) in &mut scores {
        let readiness = civilians
            .get(*actor)
            .map(|civilian| if civilian.has_moved { 0.0 } else { 0.8 })
            .unwrap_or_default();

        span.span().in_scope(|| {
            trace!("AI scorer for {:?}: {}", actor, readiness);
        });

        score.set(readiness);
    }
}

fn has_rail_target_scorer(
    civilians: Query<&Civilian>,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    capitals: Query<&Capital>,
    tile_resources: Query<&TileResource>,
    rails: Res<Rails>,
    mut scores: Query<(&Actor, &mut Score, &mut AiOrderCache, &ScorerSpan), With<HasRailTarget>>,
) {
    let tile_storage = tile_storage_query.iter().next();

    for (Actor(actor), mut score, mut cache, span) in &mut scores {
        let _guard = span.span().enter();

        let Some(storage) = tile_storage else {
            cache.rail = None;
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            cache.rail = None;
            score.set(0.0);
            continue;
        };

        if civilian.has_moved || civilian.kind != CivilianKind::Engineer {
            cache.rail = None;
            score.set(0.0);
            continue;
        }

        cache.rail = None;

        match plan_rail_connection(
            civilian,
            storage,
            &tile_provinces,
            &provinces,
            &capitals,
            &tile_resources,
            &rails,
        ) {
            Some(RailDecision::Build(target)) => {
                cache.rail = Some(CivilianOrderKind::BuildRail { to: target });
                cache.movement = None;
                score.set(0.95);
            }
            Some(RailDecision::Move(target)) => {
                cache.movement = Some(CivilianOrderKind::Move { to: target });
                score.set(0.0);
            }
            None => {
                score.set(0.0);
            }
        }
    }
}

fn has_improvement_target_scorer(
    civilians: Query<&Civilian>,
    tile_storage_query: Query<&TileStorage>,
    provinces: Query<&Province>,
    capitals: Query<&Capital>,
    tile_resources: Query<&TileResource>,
    prospecting_knowledge: Option<Res<ProspectingKnowledge>>,
    mut scores: Query<
        (&Actor, &mut Score, &mut AiOrderCache, &ScorerSpan),
        With<HasImprovementTarget>,
    >,
) {
    let tile_storage = tile_storage_query.iter().next();
    let prospecting_knowledge = prospecting_knowledge.as_deref();

    for (Actor(actor), mut score, mut cache, span) in &mut scores {
        let _guard = span.span().enter();

        let Some(storage) = tile_storage else {
            cache.improvement = None;
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            cache.improvement = None;
            score.set(0.0);
            continue;
        };

        if civilian.has_moved {
            cache.improvement = None;
            score.set(0.0);
            continue;
        }

        cache.improvement = select_improvement_target(
            civilian,
            storage,
            &provinces,
            &capitals,
            &tile_resources,
            prospecting_knowledge,
        );

        let has_target = cache.improvement.is_some();
        score.set(if has_target { 0.9 } else { 0.0 });
    }
}

fn has_move_target_scorer(
    civilians: Query<&Civilian>,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut rng: ResMut<AiRng>,
    mut scores: Query<(&Actor, &mut Score, &mut AiOrderCache, &ScorerSpan), With<HasMoveTarget>>,
) {
    let tile_storage = tile_storage_query.iter().next();

    for (Actor(actor), mut score, mut cache, span) in &mut scores {
        let _guard = span.span().enter();

        let Some(storage) = tile_storage else {
            cache.movement = None;
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            cache.movement = None;
            score.set(0.0);
            continue;
        };

        if civilian.has_moved {
            cache.movement = None;
            score.set(0.0);
            continue;
        }

        if cache.movement.is_some() {
            score.set(0.7);
            continue;
        }

        cache.movement =
            select_move_target(civilian, storage, &tile_provinces, &provinces, &mut rng.0);

        let has_target = cache.movement.is_some();
        score.set(if has_target { 0.6 } else { 0.0 });
    }
}

fn build_rail_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiOrderCache, &ActionSpan),
        With<BuildRailOrder>,
    >,
) {
    for (Actor(actor), mut state, mut cache, span) in &mut actions {
        let _guard = span.span().enter();

        match *state {
            ActionState::Init => {
                *state = ActionState::Requested;
            }
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            ActionState::Executing => {
                let Ok((civilian, pending_order)) = civilians.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if civilian.has_moved || pending_order.is_some() {
                    *state = ActionState::Success;
                    continue;
                }

                let Some(order) = cache.rail.take() else {
                    *state = ActionState::Failure;
                    continue;
                };

                command_writer.write(CivilianCommand {
                    civilian: *actor,
                    order,
                });

                cache.movement = None;
                *state = ActionState::Success;
            }
            ActionState::Success | ActionState::Failure => {}
        }
    }
}

fn build_improvement_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiOrderCache, &ActionSpan),
        With<BuildImprovementOrder>,
    >,
) {
    for (Actor(actor), mut state, mut cache, span) in &mut actions {
        let _guard = span.span().enter();

        match *state {
            ActionState::Init => {
                *state = ActionState::Requested;
            }
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            ActionState::Executing => {
                let Ok((civilian, pending_order)) = civilians.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if civilian.has_moved || pending_order.is_some() {
                    *state = ActionState::Success;
                    continue;
                }

                let Some(order) = cache.improvement.take() else {
                    *state = ActionState::Failure;
                    continue;
                };

                command_writer.write(CivilianCommand {
                    civilian: *actor,
                    order,
                });

                cache.movement = None;
                *state = ActionState::Success;
            }
            ActionState::Success | ActionState::Failure => {}
        }
    }
}

fn select_move_target(
    civilian: &Civilian,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    rng: &mut StdRng,
) -> Option<CivilianOrderKind> {
    let mut candidates: Vec<TilePos> = civilian
        .position
        .to_hex()
        .all_neighbors()
        .iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| {
            tile_owned_by_nation(*pos, civilian.owner, storage, tile_provinces, provinces)
        })
        .collect();

    candidates.shuffle(rng);
    let target = candidates.first().copied()?;

    Some(CivilianOrderKind::Move { to: target })
}

fn move_to_target_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiOrderCache, &ActionSpan),
        With<MoveTowardsOwnedTile>,
    >,
) {
    for (Actor(actor), mut state, mut cache, span) in &mut actions {
        let _guard = span.span().enter();

        match *state {
            ActionState::Init => {
                *state = ActionState::Requested;
            }
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            ActionState::Executing => {
                let Ok((civilian, pending_order)) = civilians.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if civilian.has_moved || pending_order.is_some() {
                    *state = ActionState::Success;
                    continue;
                }

                let Some(order) = cache.movement.take() else {
                    *state = ActionState::Failure;
                    continue;
                };

                command_writer.write(CivilianCommand {
                    civilian: *actor,
                    order,
                });

                cache.rail = None;
                *state = ActionState::Success;
            }
            ActionState::Success | ActionState::Failure => {}
        }
    }
}

fn skip_turn_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiOrderCache, &ActionSpan),
        With<SkipTurnOrder>,
    >,
) {
    for (Actor(actor), mut state, mut cache, span) in &mut actions {
        let _guard = span.span().enter();

        match *state {
            ActionState::Init => {
                *state = ActionState::Requested;
            }
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            ActionState::Executing => {
                let Ok((civilian, pending_order)) = civilians.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if civilian.has_moved || pending_order.is_some() {
                    *state = ActionState::Success;
                    continue;
                }

                cache.improvement = None;
                cache.rail = None;
                cache.movement = None;

                command_writer.write(CivilianCommand {
                    civilian: *actor,
                    order: CivilianOrderKind::SkipTurn,
                });

                *state = ActionState::Success;
            }
            ActionState::Success | ActionState::Failure => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RailDecision {
    Build(TilePos),
    Move(TilePos),
}

fn plan_rail_connection(
    civilian: &Civilian,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    capitals: &Query<&Capital>,
    tile_resources: &Query<&TileResource>,
    rails: &Rails,
) -> Option<RailDecision> {
    if civilian.kind != CivilianKind::Engineer {
        return None;
    }

    let capital_pos = capitals.get(civilian.owner).ok()?.0;
    let connected = gather_connected_tiles(
        capital_pos,
        civilian.owner,
        storage,
        tile_provinces,
        provinces,
        rails,
    );

    let engineer_paths = compute_owned_bfs(
        civilian.position,
        civilian.owner,
        storage,
        tile_provinces,
        provinces,
    );

    let mut best: Option<(RailDecision, (u8, u32, u32, u32))> = None;

    for province in provinces.iter() {
        if province.owner != Some(civilian.owner) {
            continue;
        }

        for tile_pos in &province.tiles {
            let Some(tile_entity) = storage.get(tile_pos) else {
                continue;
            };

            let Ok(resource) = tile_resources.get(tile_entity) else {
                continue;
            };

            if !resource.discovered || resource.development <= DevelopmentLevel::Lv0 {
                continue;
            }

            if connected.contains(tile_pos) {
                continue;
            }

            let Some(path) = shortest_path_to_connected(
                *tile_pos,
                civilian.owner,
                &connected,
                storage,
                tile_provinces,
                provinces,
            ) else {
                continue;
            };

            let path_len = path.len() as u32;

            for (index, window) in path.windows(2).enumerate() {
                let from = window[0];
                let to = window[1];

                if rails.0.contains(&ordered_edge(from, to)) {
                    continue;
                }

                let step_index = index as u32;

                if civilian.position == from {
                    let priority = (0, 0, step_index, path_len);
                    let decision = RailDecision::Build(to);
                    if best.as_ref().map(|(_, p)| priority < *p).unwrap_or(true) {
                        best = Some((decision, priority));
                    }
                    continue;
                }

                if civilian.position == to {
                    let priority = (0, 0, step_index, path_len);
                    let decision = RailDecision::Build(from);
                    if best.as_ref().map(|(_, p)| priority < *p).unwrap_or(true) {
                        best = Some((decision, priority));
                    }
                    continue;
                }

                let mut movement_choice: Option<(u32, TilePos)> = None;

                if let Some(result) = engineer_paths.first_step_towards(civilian.position, from)
                    && result.1 != civilian.position
                {
                    movement_choice = Some(result);
                }

                if let Some(result) = engineer_paths.first_step_towards(civilian.position, to) {
                    if result.1 == civilian.position {
                        continue;
                    }

                    movement_choice = match movement_choice {
                        Some(current) if result.0 < current.0 => Some(result),
                        Some(current) => Some(current),
                        None => Some(result),
                    };
                }

                let Some((distance, step)) = movement_choice else {
                    continue;
                };

                let priority = (1, distance, step_index, path_len);
                let decision = RailDecision::Move(step);

                if best.as_ref().map(|(_, p)| priority < *p).unwrap_or(true) {
                    best = Some((decision, priority));
                }
            }
        }
    }

    best.map(|(decision, _)| decision)
}

fn gather_connected_tiles(
    capital_pos: TilePos,
    owner: Entity,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    rails: &Rails,
) -> HashSet<TilePos> {
    let mut graph: HashMap<TilePos, Vec<TilePos>> = HashMap::new();

    for &(a, b) in rails.0.iter() {
        if !tile_owned_by_nation(a, owner, storage, tile_provinces, provinces)
            || !tile_owned_by_nation(b, owner, storage, tile_provinces, provinces)
        {
            continue;
        }

        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }

    let mut visited: HashSet<TilePos> = HashSet::new();
    let mut queue: VecDeque<TilePos> = VecDeque::new();

    visited.insert(capital_pos);
    queue.push_back(capital_pos);

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&current) {
            for &neighbor in neighbors {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    visited
}

fn shortest_path_to_connected(
    start: TilePos,
    owner: Entity,
    connected: &HashSet<TilePos>,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> Option<Vec<TilePos>> {
    if connected.contains(&start) {
        return None;
    }

    let mut queue: VecDeque<TilePos> = VecDeque::new();
    let mut parents: HashMap<TilePos, TilePos> = HashMap::new();
    let mut visited: HashSet<TilePos> = HashSet::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        let current_hex = current.to_hex();

        for neighbor_hex in current_hex.all_neighbors() {
            let Some(neighbor) = neighbor_hex.to_tile_pos() else {
                continue;
            };

            if !tile_owned_by_nation(neighbor, owner, storage, tile_provinces, provinces) {
                continue;
            }

            if !visited.insert(neighbor) {
                continue;
            }

            parents.insert(neighbor, current);

            if connected.contains(&neighbor) {
                return Some(reconstruct_path(start, neighbor, &parents));
            }

            queue.push_back(neighbor);
        }
    }

    None
}

fn reconstruct_path(
    start: TilePos,
    mut goal: TilePos,
    parents: &HashMap<TilePos, TilePos>,
) -> Vec<TilePos> {
    let mut path = vec![goal];

    while goal != start {
        if let Some(&parent) = parents.get(&goal) {
            goal = parent;
            path.push(goal);
        } else {
            break;
        }
    }

    path.reverse();
    path
}

#[derive(Default)]
struct OwnedBfs {
    parents: HashMap<TilePos, TilePos>,
    distances: HashMap<TilePos, u32>,
}

impl OwnedBfs {
    fn first_step_towards(&self, start: TilePos, goal: TilePos) -> Option<(u32, TilePos)> {
        if goal == start {
            return Some((0, start));
        }

        let distance = *self.distances.get(&goal)?;
        let mut current = goal;

        while let Some(&parent) = self.parents.get(&current) {
            if parent == start {
                return Some((distance, current));
            }
            current = parent;
        }

        None
    }
}

fn compute_owned_bfs(
    start: TilePos,
    owner: Entity,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> OwnedBfs {
    let mut parents: HashMap<TilePos, TilePos> = HashMap::new();
    let mut distances: HashMap<TilePos, u32> = HashMap::new();
    let mut queue: VecDeque<TilePos> = VecDeque::new();

    distances.insert(start, 0);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        let current_distance = *distances.get(&current).unwrap_or(&0);
        let current_hex = current.to_hex();

        for neighbor_hex in current_hex.all_neighbors() {
            let Some(neighbor) = neighbor_hex.to_tile_pos() else {
                continue;
            };

            if !tile_owned_by_nation(neighbor, owner, storage, tile_provinces, provinces) {
                continue;
            }

            if distances.contains_key(&neighbor) {
                continue;
            }

            parents.insert(neighbor, current);
            distances.insert(neighbor, current_distance + 1);
            queue.push_back(neighbor);
        }
    }

    OwnedBfs { parents, distances }
}

fn select_improvement_target(
    civilian: &Civilian,
    storage: &TileStorage,
    provinces: &Query<&Province>,
    capitals: &Query<&Capital>,
    tile_resources: &Query<&TileResource>,
    prospecting_knowledge: Option<&ProspectingKnowledge>,
) -> Option<CivilianOrderKind> {
    if !civilian.kind.supports_improvements() {
        return None;
    }

    let resource_predicate = civilian.kind.improvement_predicate()?;
    let capital_pos = capitals.get(civilian.owner).ok()?.0;
    let capital_hex = capital_pos.to_hex();

    let mut best_target: Option<(u32, DevelopmentLevel, TilePos)> = None;
    let mut capital_candidate: Option<(DevelopmentLevel, TilePos)> = None;

    for province in provinces.iter() {
        if province.owner != Some(civilian.owner) {
            continue;
        }

        for tile_pos in &province.tiles {
            let Some(tile_entity) = storage.get(tile_pos) else {
                continue;
            };

            let Ok(resource) = tile_resources.get(tile_entity) else {
                continue;
            };

            if !resource.discovered {
                continue;
            }

            if resource.requires_prospecting()
                && !prospecting_knowledge
                    .map(|knowledge| knowledge.is_discovered_by(tile_entity, civilian.owner))
                    .unwrap_or(false)
            {
                continue;
            }

            if !resource_predicate(resource) {
                continue;
            }

            if resource.development >= DevelopmentLevel::Lv3 {
                continue;
            }

            let distance = capital_hex.distance_to(tile_pos.to_hex()) as u32;

            if distance == 0 {
                capital_candidate = match capital_candidate {
                    Some((best_level, best_pos)) => {
                        if resource.development < best_level {
                            Some((resource.development, *tile_pos))
                        } else {
                            Some((best_level, best_pos))
                        }
                    }
                    None => Some((resource.development, *tile_pos)),
                };
                continue;
            }

            let candidate = (distance, resource.development, *tile_pos);

            best_target = match best_target {
                Some((best_distance, best_level, best_pos)) => {
                    if distance < best_distance
                        || (distance == best_distance && resource.development < best_level)
                    {
                        Some(candidate)
                    } else {
                        Some((best_distance, best_level, best_pos))
                    }
                }
                None => Some(candidate),
            };
        }
    }

    if let Some((_, _, target_pos)) = best_target {
        return civilian.kind.default_tile_action_order(target_pos);
    }

    if let Some((_, target_pos)) = capital_candidate {
        return civilian.kind.default_tile_action_order(target_pos);
    }

    None
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::{RunSystemOnce, SystemState};
    use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Update, With, Without, World};
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};
    use rand::RngCore;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::ai::behavior::{
        AiRng, RNG_BASE_SEED, RailDecision, plan_rail_connection, reset_ai_rng_on_enemy_turn,
        select_improvement_target, select_move_target, tag_ai_owned_civilians,
        untag_non_ai_owned_civilians,
    };
    use crate::ai::markers::{AiControlledCivilian, AiNation};
    use crate::civilians::types::{
        Civilian, CivilianKind, CivilianOrderKind, ProspectingKnowledge,
    };
    use crate::economy::nation::{Capital, NationId};
    use crate::economy::transport::{Rails, ordered_edge};
    use crate::map::province::{Province, ProvinceId, TileProvince};
    use crate::resources::{DevelopmentLevel, ResourceType, TileResource};
    use crate::turn_system::{TurnPhase, TurnSystem};

    #[test]
    fn tags_ai_owned_civilians() {
        let mut world = World::new();
        let ai_nation = world.spawn((AiNation(NationId(1)), NationId(1))).id();
        let civilian_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: TilePos { x: 1, y: 1 },
                owner: ai_nation,
                owner_id: NationId(1),
                selected: false,
                has_moved: false,
            })
            .id();

        let mut state: SystemState<(
            Commands,
            Query<(), With<AiNation>>,
            Query<(Entity, &Civilian), Without<AiControlledCivilian>>,
        )> = SystemState::new(&mut world);

        {
            let (commands, ai_nations, untagged) = state.get_mut(&mut world);
            tag_ai_owned_civilians(commands, ai_nations, untagged);
        }
        state.apply(&mut world);

        assert!(world.get::<AiControlledCivilian>(civilian_entity).is_some());
    }

    #[test]
    fn removes_tag_from_non_ai_owned_civilians() {
        let mut world = World::new();
        let player_nation = world.spawn(NationId(2)).id();
        let civilian_entity = world
            .spawn((
                Civilian {
                    kind: CivilianKind::Engineer,
                    position: TilePos { x: 1, y: 1 },
                    owner: player_nation,
                    owner_id: NationId(2),
                    selected: false,
                    has_moved: false,
                },
                AiControlledCivilian,
            ))
            .id();

        let mut state: SystemState<(
            Commands,
            Query<(), With<AiNation>>,
            Query<(Entity, &Civilian), With<AiControlledCivilian>>,
        )> = SystemState::new(&mut world);

        {
            let (commands, ai_nations, tagged) = state.get_mut(&mut world);
            untag_non_ai_owned_civilians(commands, ai_nations, tagged);
        }
        state.apply(&mut world);

        assert!(world.get::<AiControlledCivilian>(civilian_entity).is_none());
    }

    #[test]
    fn engineer_plans_build_for_unconnected_improvement() {
        let mut world = World::new();
        world.insert_resource(Rails::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let neighbor_pos = TilePos { x: 2, y: 1 };
        let improvement_pos = TilePos { x: 3, y: 1 };
        let province_id = ProvinceId(1);

        let ai_nation = world
            .spawn((AiNation(NationId(3)), NationId(3), Capital(capital_pos)))
            .id();

        {
            let mut rails = world.resource_mut::<Rails>();
            rails.0.insert(ordered_edge(capital_pos, neighbor_pos));
        }

        let mut storage = TileStorage::empty(TilemapSize { x: 6, y: 6 });

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv0,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let neighbor_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&neighbor_pos, neighbor_tile);

        let improvement_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&improvement_pos, improvement_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, neighbor_pos, improvement_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let engineer_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: neighbor_pos,
                owner: ai_nation,
                owner_id: NationId(3),
                selected: false,
                has_moved: false,
            })
            .id();

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Res<Rails>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (tile_provinces, provinces, capitals, tile_resources, rails, civilians) =
                state.get(&mut world);
            let civilian = civilians.get(engineer_entity).unwrap();
            plan_rail_connection(
                civilian,
                &storage,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &rails,
            )
        };

        match decision {
            Some(RailDecision::Build(target)) => assert_eq!(target, improvement_pos),
            other => panic!("expected build decision, got {:?}", other),
        }
    }

    #[test]
    fn engineer_plans_move_toward_unconnected_improvement() {
        let mut world = World::new();
        world.insert_resource(Rails::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let neighbor_pos = TilePos { x: 2, y: 1 };
        let improvement_pos = TilePos { x: 3, y: 1 };
        let province_id = ProvinceId(1);

        let ai_nation = world
            .spawn((AiNation(NationId(4)), NationId(4), Capital(capital_pos)))
            .id();

        {
            let mut rails = world.resource_mut::<Rails>();
            rails.0.insert(ordered_edge(capital_pos, neighbor_pos));
        }

        let mut storage = TileStorage::empty(TilemapSize { x: 6, y: 6 });

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv0,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let neighbor_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&neighbor_pos, neighbor_tile);

        let improvement_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&improvement_pos, improvement_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, neighbor_pos, improvement_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let engineer_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: capital_pos,
                owner: ai_nation,
                owner_id: NationId(4),
                selected: false,
                has_moved: false,
            })
            .id();

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Res<Rails>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (tile_provinces, provinces, capitals, tile_resources, rails, civilians) =
                state.get(&mut world);
            let civilian = civilians.get(engineer_entity).unwrap();
            plan_rail_connection(
                civilian,
                &storage,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &rails,
            )
        };

        match decision {
            Some(RailDecision::Move(target)) => assert_eq!(target, neighbor_pos),
            other => panic!("expected move decision, got {:?}", other),
        }
    }

    #[test]
    fn reseeds_rng_when_enemy_turn_begins() {
        let mut world = World::new();
        world.insert_resource(AiRng(StdRng::seed_from_u64(0xDEADBEEF)));
        world.insert_resource(TurnSystem {
            current_turn: 7,
            phase: TurnPhase::EnemyTurn,
        });

        let mut state: SystemState<(ResMut<AiRng>, Res<TurnSystem>)> = SystemState::new(&mut world);

        {
            let (rng, turn) = state.get_mut(&mut world);
            reset_ai_rng_on_enemy_turn(rng, turn);
        }
        state.apply(&mut world);

        let mut expected = StdRng::seed_from_u64(RNG_BASE_SEED ^ 7);
        let mut rng = world.resource_mut::<AiRng>();
        let actual = rng.0.next_u32();
        let expected_value = expected.next_u32();
        assert_eq!(actual, expected_value);
    }

    #[test]
    fn selects_owned_neighbor_as_move_target() {
        let mut world = World::new();
        let ai_nation = world.spawn((AiNation(NationId(5)), NationId(5))).id();
        let neighbor_pos = TilePos { x: 2, y: 1 };
        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });

        let province_id = ProvinceId(1);
        let start_tile = world.spawn(TileProvince { province_id }).id();
        let neighbor_tile = world.spawn(TileProvince { province_id }).id();

        storage.set(&TilePos { x: 1, y: 1 }, start_tile);
        storage.set(&neighbor_pos, neighbor_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![TilePos { x: 1, y: 1 }, neighbor_pos],
            city_tile: TilePos { x: 1, y: 1 },
            owner: Some(ai_nation),
        });

        let civilian = Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 1, y: 1 },
            owner: ai_nation,
            owner_id: NationId(5),
            selected: false,
            has_moved: false,
        };

        let mut state: SystemState<(Query<&TileProvince>, Query<&Province>)> =
            SystemState::new(&mut world);

        let mut rng = StdRng::seed_from_u64(42);
        let order = {
            let (tile_provinces, provinces) = state.get(&mut world);
            let order =
                select_move_target(&civilian, &storage, &tile_provinces, &provinces, &mut rng);
            order
        };
        state.apply(&mut world);

        match order {
            Some(CivilianOrderKind::Move { to }) => assert_eq!(to, neighbor_pos),
            _ => panic!("expected move order"),
        }
    }

    #[test]
    fn selects_improvement_adjacent_to_capital() {
        let mut world = World::new();
        world.insert_resource(ProspectingKnowledge::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let ai_nation = world
            .spawn((AiNation(NationId(6)), NationId(6), Capital(capital_pos)))
            .id();

        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });
        let province_id = ProvinceId(1);

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Grain),
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let neighbor_pos = TilePos { x: 2, y: 1 };
        let neighbor_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Cotton),
            ))
            .id();
        storage.set(&neighbor_pos, neighbor_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, neighbor_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let civilian = Civilian {
            kind: CivilianKind::Farmer,
            position: capital_pos,
            owner: ai_nation,
            owner_id: NationId(6),
            selected: false,
            has_moved: false,
        };

        let mut state: SystemState<(
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Res<ProspectingKnowledge>,
        )> = SystemState::new(&mut world);

        let order = {
            let (provinces, capitals, tile_resources, knowledge) = state.get(&mut world);
            select_improvement_target(
                &civilian,
                &storage,
                &provinces,
                &capitals,
                &tile_resources,
                Some(&*knowledge),
            )
        };
        state.apply(&mut world);

        match order {
            Some(CivilianOrderKind::ImproveTile { to }) => assert_eq!(to, neighbor_pos),
            other => panic!("expected improvement order, got {:?}", other),
        }
    }

    #[test]
    fn skips_mineral_without_prospecting_knowledge() {
        let mut world = World::new();
        world.insert_resource(ProspectingKnowledge::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let ai_nation = world
            .spawn((AiNation(NationId(7)), NationId(7), Capital(capital_pos)))
            .id();

        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });
        let province_id = ProvinceId(1);

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Grain),
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let mineral_pos = TilePos { x: 2, y: 1 };
        let mineral_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Coal),
            ))
            .id();
        storage.set(&mineral_pos, mineral_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, mineral_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let civilian = Civilian {
            kind: CivilianKind::Miner,
            position: capital_pos,
            owner: ai_nation,
            owner_id: NationId(7),
            selected: false,
            has_moved: false,
        };

        let mut state: SystemState<(
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Res<ProspectingKnowledge>,
        )> = SystemState::new(&mut world);

        let order = {
            let (provinces, capitals, tile_resources, knowledge) = state.get(&mut world);
            select_improvement_target(
                &civilian,
                &storage,
                &provinces,
                &capitals,
                &tile_resources,
                Some(&*knowledge),
            )
        };
        state.apply(&mut world);

        assert!(
            order.is_none(),
            "miner should wait for prospecting knowledge"
        );
    }

    #[test]
    fn selects_mineral_once_prospected() {
        let mut world = World::new();
        world.insert_resource(ProspectingKnowledge::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let ai_nation = world
            .spawn((AiNation(NationId(8)), NationId(8), Capital(capital_pos)))
            .id();

        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });
        let province_id = ProvinceId(1);

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Grain),
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let mineral_pos = TilePos { x: 2, y: 1 };
        let mineral_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource::visible(ResourceType::Coal),
            ))
            .id();
        storage.set(&mineral_pos, mineral_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, mineral_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        {
            let mut knowledge = world.resource_mut::<ProspectingKnowledge>();
            knowledge.mark_discovered(mineral_tile, ai_nation);
        }

        let civilian = Civilian {
            kind: CivilianKind::Miner,
            position: capital_pos,
            owner: ai_nation,
            owner_id: NationId(8),
            selected: false,
            has_moved: false,
        };

        let mut state: SystemState<(
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Res<ProspectingKnowledge>,
        )> = SystemState::new(&mut world);

        let order = {
            let (provinces, capitals, tile_resources, knowledge) = state.get(&mut world);
            select_improvement_target(
                &civilian,
                &storage,
                &provinces,
                &capitals,
                &tile_resources,
                Some(&*knowledge),
            )
        };
        state.apply(&mut world);

        match order {
            Some(CivilianOrderKind::Mine { to }) => assert_eq!(to, mineral_pos),
            other => panic!("expected mining order, got {:?}", other),
        }
    }

    #[test]
    fn test_reset_ai_action_states_resets_success_to_init() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(bevy::prelude::Update));

        // Spawn an AI civilian action in Success state
        let civilian_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Success,
                super::SkipTurnOrder,
            ))
            .id();

        // Spawn an AI economy action in Success state
        let economy_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Success,
                crate::ai::trade::PlanBuildingFocus,
            ))
            .id();

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify both were reset to Init
        let civilian_state = app.world().get::<ActionState>(civilian_action).unwrap();
        assert_eq!(
            *civilian_state,
            ActionState::Init,
            "Civilian action should be reset to Init"
        );

        let economy_state = app.world().get::<ActionState>(economy_action).unwrap();
        assert_eq!(
            *economy_state,
            ActionState::Init,
            "Economy action should be reset to Init"
        );
    }

    #[test]
    fn test_reset_ai_action_states_resets_failure_to_init() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(bevy::prelude::Update));

        // Spawn actions in Failure state
        let action1 = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Failure,
                super::BuildRailOrder,
            ))
            .id();

        let action2 = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Failure,
                crate::ai::trade::ApplyProductionPlan,
            ))
            .id();

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify both were reset
        assert_eq!(
            *app.world().get::<ActionState>(action1).unwrap(),
            ActionState::Init
        );
        assert_eq!(
            *app.world().get::<ActionState>(action2).unwrap(),
            ActionState::Init
        );
    }

    #[test]
    fn test_reset_ai_action_states_preserves_other_states() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(bevy::prelude::Update));

        // Spawn actions in various non-terminal states
        let init_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Init,
                super::SkipTurnOrder,
            ))
            .id();

        let requested_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Requested,
                super::BuildImprovementOrder,
            ))
            .id();

        let executing_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Executing,
                super::MoveTowardsOwnedTile,
            ))
            .id();

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify non-terminal states are unchanged
        assert_eq!(
            *app.world().get::<ActionState>(init_action).unwrap(),
            ActionState::Init,
            "Init should remain Init"
        );
        assert_eq!(
            *app.world().get::<ActionState>(requested_action).unwrap(),
            ActionState::Requested,
            "Requested should remain Requested"
        );
        assert_eq!(
            *app.world().get::<ActionState>(executing_action).unwrap(),
            ActionState::Executing,
            "Executing should remain Executing"
        );
    }

    #[test]
    fn test_reset_ai_action_states_ignores_non_actor_actions() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(bevy::prelude::Update));

        // Spawn an action without Actor component (shouldn't be reset)
        let non_actor_action = app
            .world_mut()
            .spawn((ActionState::Success, super::SkipTurnOrder))
            .id();

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify it was not reset (still Success)
        assert_eq!(
            *app.world().get::<ActionState>(non_actor_action).unwrap(),
            ActionState::Success,
            "Non-Actor action should not be reset"
        );
    }

    #[test]
    fn test_reset_respects_actor_component() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(Update));

        // Spawn an action WITH Actor component
        let actor_action = app
            .world_mut()
            .spawn((
                Actor(Entity::PLACEHOLDER),
                ActionState::Success,
                super::SkipTurnOrder,
            ))
            .id();

        // Spawn an action WITHOUT Actor component
        let non_actor_action = app
            .world_mut()
            .spawn((ActionState::Success, super::SkipTurnOrder))
            .id();

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify actor action was reset
        assert_eq!(
            *app.world().get::<ActionState>(actor_action).unwrap(),
            ActionState::Init,
            "Action with Actor should be reset"
        );

        // Verify non-actor action was NOT reset
        assert_eq!(
            *app.world().get::<ActionState>(non_actor_action).unwrap(),
            ActionState::Success,
            "Action without Actor should not be reset"
        );
    }

    #[test]
    fn test_multiple_ai_actions_reset_together() {
        use bevy::prelude::App;
        use big_brain::prelude::*;

        let mut app = App::new();
        app.add_plugins(BigBrainPlugin::new(bevy::prelude::Update));

        // Spawn multiple AI actions of different types, all in Success
        let actions: Vec<Entity> = vec![
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    super::SkipTurnOrder,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    super::BuildRailOrder,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    super::BuildImprovementOrder,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    super::MoveTowardsOwnedTile,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    crate::ai::trade::PlanBuildingFocus,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    crate::ai::trade::ApplyProductionPlan,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    crate::ai::trade::PlanMarketOrders,
                ))
                .id(),
            app.world_mut()
                .spawn((
                    Actor(Entity::PLACEHOLDER),
                    ActionState::Success,
                    crate::ai::trade::EconomyIdleAction,
                ))
                .id(),
        ];

        // Run the reset system
        app.world_mut()
            .run_system_once(super::reset_ai_action_states)
            .unwrap();

        // Verify all were reset
        for action in actions {
            assert_eq!(
                *app.world().get::<ActionState>(action).unwrap(),
                ActionState::Init,
                "All AI actions should be reset to Init"
            );
        }
    }
}
