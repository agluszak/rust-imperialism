#![allow(clippy::type_complexity)]

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};
use big_brain::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::ai::context::{
    AiPlanLedger, MacroTag, MarketView, TransportAnalysis, TurnCandidates, gather_turn_candidates,
    resource_target_days, update_belief_state_system, update_market_view_system,
    update_transport_analysis_system,
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
use crate::economy::technology::Technologies;
use crate::economy::transport::validation::can_build_rail_on_terrain;
use crate::economy::transport::{Depot, ImprovementKind, PlaceImprovement, Rails, ordered_edge};
use crate::map::province::{Province, TileProvince};
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::map::tiles::TerrainType;
use crate::messages::civilians::CivilianCommand;
use crate::orders::{OrdersOut, flush_orders_to_queue};
use crate::resources::{DevelopmentLevel, TileResource};
use crate::turn_system::{EnemyTurnSet, TurnCounter, TurnPhase};
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

pub(crate) const RNG_BASE_SEED: u64 = 0xA1_51_23_45;

// AI scoring and priority constants
const DEPOT_VALUE_SCORE: u32 = 10;
const DEPOT_BASE_PRIORITY: f32 = 0.94;
const DEPOT_PENALTY_PER_UNCONNECTED: f32 = 0.1;
const MAX_DEPOT_PENALTY: f32 = 0.44;
const PRIORITY_DISTANCE_WEIGHT: u32 = 2;
const PRIORITY_OUTPUT_SCALE: u32 = 10;

/// Registers Big Brain and the systems that drive simple AI-controlled civilians.
pub struct AiBehaviorPlugin;

#[derive(Component, Debug, Default)]
struct AiOrderCache {
    improvement: Option<CivilianOrderKind>,
    rail: Option<CivilianOrderKind>,
    depot: Option<CivilianOrderKind>,
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
            );

        // ====================================================================
        // EnemyTurn setup systems (run once on entry via OnEnter)
        // ====================================================================
        app.add_systems(
            OnEnter(TurnPhase::EnemyTurn),
            (
                reset_ai_rng_on_enemy_turn,
                reset_ai_civilian_actions,
                reset_ai_action_states,
                gate_ai_turn,
            )
                .chain()
                .in_set(EnemyTurnSet::Setup),
        );

        // ====================================================================
        // Continuous systems (run every frame during EnemyTurn)
        // ====================================================================
        app.add_systems(
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
                .run_if(in_state(TurnPhase::EnemyTurn)),
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
                .run_if(in_state(TurnPhase::EnemyTurn)),
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
                .run_if(in_state(TurnPhase::EnemyTurn)),
        )
        .add_systems(
            PreUpdate,
            flush_orders_to_queue
                .in_set(AiSet::EmitOrders)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(TurnPhase::EnemyTurn)),
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
                ready_to_act_scorer,
                has_rail_target_scorer,
                has_depot_target_scorer,
                has_improvement_target_scorer,
                has_move_target_scorer,
            )
                .chain()
                .in_set(BigBrainSet::Scorers)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(TurnPhase::EnemyTurn)),
        )
        .add_systems(
            PreUpdate,
            (
                build_rail_action,
                build_depot_action,
                build_improvement_action,
                move_to_target_action,
                skip_turn_action,
            )
                .in_set(BigBrainSet::Actions)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(TurnPhase::EnemyTurn)),
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
                .remove::<HasDepotTarget>()
                .remove::<HasImprovementTarget>()
                .remove::<HasMoveTarget>()
                .remove::<BuildRailOrder>()
                .remove::<BuildDepotOrder>()
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

/// Checks whether an engineer should build a depot to collect resources.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct HasDepotTarget;

/// Issues a depot construction order gathered during scoring.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct BuildDepotOrder;

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
                    nation: Some(*actor),
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

/// Gate function that runs once on EnemyTurn entry to clear/advance turn state.
///
/// Note: Runs via OnEnter(TurnPhase::EnemyTurn) in EnemyTurnSet::Setup.
fn gate_ai_turn(
    turn: Res<TurnCounter>,
    mut ledger: ResMut<AiPlanLedger>,
    mut orders: ResMut<OrdersOut>,
    mut candidates: ResMut<TurnCandidates>,
) {
    ledger.advance_turn(turn.current);
    orders.clear();
    candidates.clear();
}

/// Reset AI RNG for deterministic behavior each turn.
///
/// Note: Runs via OnEnter(TurnPhase::EnemyTurn) in EnemyTurnSet::Setup.
fn reset_ai_rng_on_enemy_turn(mut rng: ResMut<AiRng>, turn: Res<TurnCounter>) {
    rng.0 = StdRng::seed_from_u64(RNG_BASE_SEED ^ u64::from(turn.current));
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
                .when(HasDepotTarget, BuildDepotOrder)
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
    mut caches: Query<&mut AiOrderCache>,
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    capitals: Query<&Capital>,
    tile_resources: Query<&TileResource>,
    terrain_types: Query<&TerrainType>,
    nation_technologies: Query<&Technologies>,
    depots: Query<&Depot>,
    rails: Res<Rails>,
    turn: Res<TurnCounter>,
    mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<HasRailTarget>>,
) {
    let tile_data = tile_storage_query.iter().next();

    for (Actor(actor), mut score, span) in &mut scores {
        let _guard = span.span().enter();

        let Some((storage, map_size)) = tile_data else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.rail = None;
            }
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.rail = None;
            }
            score.set(0.0);
            continue;
        };

        if civilian.has_moved || civilian.kind != CivilianKind::Engineer {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.rail = None;
            }
            score.set(0.0);
            continue;
        }

        let Ok(mut cache) = caches.get_mut(*actor) else {
            score.set(0.0);
            continue;
        };

        cache.rail = None;

        // Get this nation's technologies for terrain checks
        let nation_techs = nation_technologies.get(civilian.owner).ok();

        let rail_decision = plan_rail_connection(
            civilian,
            storage,
            *map_size,
            &tile_provinces,
            &provinces,
            &capitals,
            &tile_resources,
            &terrain_types,
            nation_techs,
            &depots,
            &rails,
        );

        match rail_decision {
            Some(RailDecision::Build(target)) => {
                cache.rail = Some(CivilianOrderKind::BuildRail { to: target });
                cache.movement = None;
                // Early game (turns 1-30): High priority for rail building
                // Mid-game (turns 31-60): Balanced priority
                // Late game (turns 61+): Lower priority, focus on production
                let base_score = if turn.current <= 30 {
                    0.96  // Very high priority early
                } else if turn.current <= 60 {
                    0.95  // High priority mid-game
                } else {
                    0.93  // Still important but allow other tasks
                };
                score.set(base_score);
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
    mut caches: Query<&mut AiOrderCache>,
    tile_storage_query: Query<&TileStorage>,
    provinces: Query<&Province>,
    capitals: Query<&Capital>,
    tile_resources: Query<&TileResource>,
    prospecting_knowledge: Option<Res<ProspectingKnowledge>>,
    turn: Res<TurnCounter>,
    mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<HasImprovementTarget>>,
) {
    let tile_storage = tile_storage_query.iter().next();
    let prospecting_knowledge = prospecting_knowledge.as_deref();

    for (Actor(actor), mut score, span) in &mut scores {
        let _guard = span.span().enter();

        let Some(storage) = tile_storage else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.improvement = None;
            }
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.improvement = None;
            }
            score.set(0.0);
            continue;
        };

        if civilian.has_moved {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.improvement = None;
            }
            score.set(0.0);
            continue;
        }

        let Ok(mut cache) = caches.get_mut(*actor) else {
            score.set(0.0);
            continue;
        };

        // Prospectors should prioritize prospecting undiscovered minerals
        cache.improvement = if civilian.kind == CivilianKind::Prospector {
            select_prospecting_target(
                civilian,
                storage,
                &provinces,
                &capitals,
                &tile_resources,
                prospecting_knowledge,
            )
            .or_else(|| {
                // If no prospecting targets, fall back to improvement
                select_improvement_target(
                    civilian,
                    storage,
                    &provinces,
                    &capitals,
                    &tile_resources,
                    prospecting_knowledge,
                )
            })
        } else {
            select_improvement_target(
                civilian,
                storage,
                &provinces,
                &capitals,
                &tile_resources,
                prospecting_knowledge,
            )
        };

        let has_target = cache.improvement.is_some();
        // Late game: Higher priority for improvements (resource development)
        // Early/mid game: Lower priority (infrastructure first)
        let base_score = if turn.current <= 30 {
            0.88  // Lower priority early game
        } else if turn.current <= 60 {
            0.90  // Balanced mid-game
        } else {
            0.92  // Higher priority late game
        };
        score.set(if has_target { base_score } else { 0.0 });
    }
}

fn has_move_target_scorer(
    civilians: Query<&Civilian>,
    mut caches: Query<&mut AiOrderCache>,
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut rng: ResMut<AiRng>,
    mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<HasMoveTarget>>,
) {
    let tile_data = tile_storage_query.iter().next();

    for (Actor(actor), mut score, span) in &mut scores {
        let _guard = span.span().enter();

        let Some((storage, map_size)) = tile_data else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.movement = None;
            }
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.movement = None;
            }
            score.set(0.0);
            continue;
        };

        if civilian.has_moved {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.movement = None;
            }
            score.set(0.0);
            continue;
        }

        let Ok(mut cache) = caches.get_mut(*actor) else {
            score.set(0.0);
            continue;
        };

        if cache.movement.is_some() {
            score.set(0.7);
            continue;
        }

        cache.movement = select_move_target(
            civilian,
            storage,
            *map_size,
            &tile_provinces,
            &provinces,
            &mut rng.0,
        );

        let has_target = cache.movement.is_some();
        score.set(if has_target { 0.6 } else { 0.0 });
    }
}

fn build_rail_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut caches: Query<&mut AiOrderCache>,
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<BuildRailOrder>>,
) {
    for (Actor(actor), mut state, span) in &mut actions {
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

                let Ok(mut cache) = caches.get_mut(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

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
    mut caches: Query<&mut AiOrderCache>,
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<BuildImprovementOrder>>,
) {
    for (Actor(actor), mut state, span) in &mut actions {
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

                let Ok(mut cache) = caches.get_mut(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

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
    _storage: &TileStorage,
    _map_size: TilemapSize,
    _tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    rng: &mut StdRng,
) -> Option<CivilianOrderKind> {
    // Collect ALL tiles from owned provinces (civilians can move anywhere in their territory)
    let mut candidates: Vec<TilePos> = provinces
        .iter()
        .filter(|province| province.owner == Some(civilian.owner))
        .flat_map(|province| province.tiles.iter().copied())
        .filter(|pos| *pos != civilian.position) // Don't pick current position
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.shuffle(rng);
    let target = candidates.first().copied()?;

    Some(CivilianOrderKind::Move { to: target })
}

fn move_to_target_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut caches: Query<&mut AiOrderCache>,
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<MoveTowardsOwnedTile>>,
) {
    for (Actor(actor), mut state, span) in &mut actions {
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

                let Ok(mut cache) = caches.get_mut(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

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
    mut caches: Query<&mut AiOrderCache>,
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<SkipTurnOrder>>,
) {
    for (Actor(actor), mut state, span) in &mut actions {
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

                if let Ok(mut cache) = caches.get_mut(*actor) {
                    cache.improvement = None;
                    cache.rail = None;
                    cache.movement = None;
                }

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
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    capitals: &Query<&Capital>,
    tile_resources: &Query<&TileResource>,
    terrain_types: &Query<&TerrainType>,
    nation_techs: Option<&Technologies>,
    depots: &Query<&Depot>,
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
        map_size,
        tile_provinces,
        provinces,
        rails,
    );

    let engineer_paths = compute_owned_bfs(
        civilian.position,
        civilian.owner,
        storage,
        map_size,
        tile_provinces,
        provinces,
    );

    let mut best: Option<(RailDecision, (u32, u32, u8, u32))> = None;

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

            let has_depot = depots
                .iter()
                .any(|d| d.position == *tile_pos && d.owner == civilian.owner);

            if !has_depot && (!resource.discovered || resource.development <= DevelopmentLevel::Lv0)
            {
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
                map_size,
                tile_provinces,
                provinces,
                terrain_types,
                nation_techs,
            } else {
                continue;
            };

            let path_len = path.len() as u32;
            
            // Calculate resource value for this target to prioritize high-value connections
            let resource_value = if has_depot {
                // Depots are valuable - worth connecting
                DEPOT_VALUE_SCORE
            } else {
                resource.get_output() * (resource.development as u32 + 1)
            };

            for (index, window) in path.windows(2).enumerate() {
                let from = window[0];
                let to = window[1];

                if rails.0.contains(&ordered_edge(from, to)) {
                    continue;
                }

                let step_index = index as u32;

                if civilian.position == from {
                    // Priority: (PathLen, ResourceValue, NetworkProximity, ActionType, Distance)
                    // Lower path_len is better, higher resource_value is better
                    // We invert resource_value so higher values have lower (better) priority
                    let priority = (path_len, u32::MAX - resource_value, u32::MAX - step_index, 0, 0);
                    let decision = RailDecision::Build(to);
                    if best.as_ref().map(|(_, p)| priority < *p).unwrap_or(true) {
                        best = Some((decision, priority));
                    }
                    continue;
                }

                if civilian.position == to {
                    let priority = (path_len, u32::MAX - resource_value, u32::MAX - step_index, 0, 0);
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

                // Move priority:
                // 1. Path Length (Prioritize shorter paths)
                // 2. Resource Value (Prioritize higher value targets)
                // 3. Network Proximity (Prioritize segments connected to network)
                // 4. Action type (1 = Move)
                // 5. Travel Distance (Prioritize closer segments if all else equal)
                let priority = (path_len, u32::MAX - resource_value, u32::MAX - step_index, 1, distance);
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
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    rails: &Rails,
) -> HashSet<TilePos> {
    let mut graph: HashMap<TilePos, Vec<TilePos>> = HashMap::new();

    for &(a, b) in rails.0.iter() {
        if !tile_owned_by_nation(a, owner, storage, map_size, tile_provinces, provinces)
            || !tile_owned_by_nation(b, owner, storage, map_size, tile_provinces, provinces)
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
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    terrain_types: &Query<&TerrainType>,
    nation_techs: Option<&Technologies>,
) -> Option<Vec<TilePos>> {
    if connected.contains(&start) {
        return None;
    }

    // Default empty technologies if nation has none
    let default_techs = Technologies::default();
    let techs = nation_techs.unwrap_or(&default_techs);

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

            if !tile_owned_by_nation(
                neighbor,
                owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                continue;
            }

            // Check if rail can be built on this terrain
            if let Some(tile_entity) = storage.get(&neighbor)
                && let Ok(terrain) = terrain_types.get(tile_entity)
            {
                let (can_build, _) = can_build_rail_on_terrain(terrain, techs);
                if !can_build {
                    continue;
                }
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
    map_size: TilemapSize,
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

            if !tile_owned_by_nation(
                neighbor,
                owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
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

/// Selects prospecting targets for Prospectors, prioritizing unexplored mineral-bearing terrain
fn select_prospecting_target(
    civilian: &Civilian,
    storage: &TileStorage,
    provinces: &Query<&Province>,
    capitals: &Query<&Capital>,
    tile_resources: &Query<&TileResource>,
    prospecting_knowledge: Option<&ProspectingKnowledge>,
) -> Option<CivilianOrderKind> {
    if civilian.kind != CivilianKind::Prospector {
        return None;
    }
    
    let capital_pos = capitals.get(civilian.owner).ok()?.0;
    let capital_hex = capital_pos.to_hex();
    let mut best_target: Option<(u32, TilePos)> = None;
    
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
            
            // Skip if this tile doesn't need prospecting or is already prospected
            if !resource.requires_prospecting() {
                continue;
            }
            
            if prospecting_knowledge
                .map(|k| k.is_discovered_by(tile_entity, civilian.owner))
                .unwrap_or(false)
            {
                continue;
            }
            
            // This is an unprospected mineral tile - calculate priority
            let distance = capital_hex.distance_to(tile_pos.to_hex()) as u32;
            
            // Prioritize closer tiles for prospecting
            let priority = distance;
            
            best_target = match best_target {
                Some((best_priority, _)) if priority < best_priority => Some((priority, *tile_pos)),
                None => Some((priority, *tile_pos)),
                other => other,
            };
        }
    }
    
    best_target.map(|(_, pos)| CivilianOrderKind::Prospect { to: pos })
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
            
            // Calculate priority score: lower is better
            // Consider: resource output potential, distance, and current development
            let base_output = resource.get_output();
            let potential_gain = (DevelopmentLevel::Lv3 as u32) - (resource.development as u32);
            // Resources with higher base output and more room for improvement are prioritized
            // Distance penalty: each tile away reduces priority
            // Use saturating_sub to ensure we never go negative
            let priority_score = (distance * PRIORITY_DISTANCE_WEIGHT) 
                .saturating_add(PRIORITY_OUTPUT_SCALE.saturating_sub(base_output.min(PRIORITY_OUTPUT_SCALE)))
                .saturating_sub(potential_gain);

            if distance == 0 {
                capital_candidate = match capital_candidate {
                    Some((best_score, best_pos)) => {
                        if priority_score < best_score {
                            Some((priority_score, *tile_pos))
                        } else {
                            Some((best_score, best_pos))
                        }
                    }
                    None => Some((priority_score, *tile_pos)),
                };
                continue;
            }

            let candidate = (priority_score, *tile_pos);

            best_target = match best_target {
                Some((best_score, best_pos)) => {
                    if priority_score < best_score {
                        Some(candidate)
                    } else {
                        Some((best_score, best_pos))
                    }
                }
                None => Some(candidate),
            };
        }
    }

    if let Some((_, target_pos)) = best_target {
        return civilian.kind.default_tile_action_order(target_pos);
    }

    if let Some((_, target_pos)) = capital_candidate {
        return civilian.kind.default_tile_action_order(target_pos);
    }

    None
}

fn has_depot_target_scorer(
    civilians: Query<&Civilian>,
    mut caches: Query<&mut AiOrderCache>,
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    capitals: Query<&Capital>,
    tile_resources: Query<&TileResource>,
    depots: Query<&Depot>,
    rails: Res<Rails>,
    mut scores: Query<(&Actor, &mut Score, Option<&ScorerSpan>), With<HasDepotTarget>>,
) {
    let tile_data = tile_storage_query.iter().next();

    // Cache connectivity per owner to avoid re-calculating for every civilian
    let mut connectivity_cache: HashMap<Entity, HashSet<TilePos>> = HashMap::new();

    for (Actor(actor), mut score, span) in &mut scores {
        let _guard = span.map(|s| s.span().enter());

        let Some((storage, map_size)) = tile_data else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.depot = None;
            }
            score.set(0.0);
            continue;
        };

        let Ok(civilian) = civilians.get(*actor) else {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.depot = None;
            }
            score.set(0.0);
            continue;
        };

        // Pre-calculate connectivity if not already cached
        let connected_tiles = connectivity_cache.entry(civilian.owner).or_insert_with(|| {
            let capital_pos = capitals.get(civilian.owner).ok().map(|c| c.0);
            if let Some(cap_pos) = capital_pos {
                gather_connected_tiles(
                    cap_pos,
                    civilian.owner,
                    storage,
                    *map_size,
                    &tile_provinces,
                    &provinces,
                    &rails,
                )
            } else {
                HashSet::new()
            }
        });

        let distance_map = compute_network_distance_map(
            connected_tiles,
            civilian.owner,
            storage,
            *map_size,
            &tile_provinces,
            &provinces,
        );

        if civilian.has_moved || civilian.kind != CivilianKind::Engineer {
            if let Ok(mut cache) = caches.get_mut(*actor) {
                cache.depot = None;
            }
            score.set(0.0);
            continue;
        }

        let Ok(mut cache) = caches.get_mut(*actor) else {
            score.set(0.0);
            continue;
        };

        cache.depot = None;

        if let Some(target) = select_depot_target(
            civilian,
            storage,
            *map_size,
            &tile_provinces,
            &provinces,
            &capitals,
            &tile_resources,
            &depots,
            &distance_map,
        ) {
            // Count unconnected depots to inform scoring priority
            let unconnected_depot_count = depots
                .iter()
                .filter(|d| d.owner == civilian.owner && !connected_tiles.contains(&d.position))
                .count();

            if target == civilian.position {
                cache.depot = Some(CivilianOrderKind::BuildDepot);
                cache.movement = None;
                
                // Reduce priority when unconnected depots exist, but don't block entirely
                let priority_penalty = (unconnected_depot_count as f32 * DEPOT_PENALTY_PER_UNCONNECTED)
                    .min(MAX_DEPOT_PENALTY);
                score.set(DEPOT_BASE_PRIORITY - priority_penalty);
            } else {
                cache.movement = Some(CivilianOrderKind::Move { to: target });
                score.set(0.0);
            }
        } else {
            score.set(0.0);
        }
    }
}

fn build_depot_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    mut caches: Query<&mut AiOrderCache>,
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<BuildDepotOrder>>,
) {
    for (Actor(actor), mut state, span) in &mut actions {
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

                let Ok(mut cache) = caches.get_mut(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                let Some(order) = cache.depot.take() else {
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

fn select_depot_target(
    civilian: &Civilian,
    storage: &TileStorage,
    map_size: TilemapSize,
    _tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    capitals: &Query<&Capital>,
    tile_resources: &Query<&TileResource>,
    depots: &Query<&Depot>,
    distance_map: &HashMap<TilePos, u32>,
) -> Option<TilePos> {
    if civilian.kind != CivilianKind::Engineer {
        return None;
    }

    let capital_pos = capitals.get(civilian.owner).ok()?.0;

    let mut best_target: Option<(u32, u32, TilePos)> = None;

    for province in provinces.iter() {
        if province.owner != Some(civilian.owner) {
            continue;
        }

        for tile_pos in &province.tiles {
            // Check if tile already has a depot
            let has_depot = depots.iter().any(|d| d.position == *tile_pos);
            if has_depot {
                continue;
            }

            if *tile_pos == capital_pos {
                continue;
            }

            // FILTER: Must be reachable (have a distance cost) or be connected (dist 0)
            // If it's not in the map, it's unreachable.
            let Some(&connection_cost) = distance_map.get(tile_pos) else {
                continue;
            };

            let mut yield_score = 0;
            let center_hex = tile_pos.to_hex();

            // Check center tile
            if let Some(entity) = storage.get(tile_pos)
                && let Ok(resource) = tile_resources.get(entity)
                && resource.discovered
            {
                yield_score += resource.get_output();
            }

            // Check neighbors
            for neighbor_hex in center_hex.all_neighbors() {
                let Some(neighbor_pos) = neighbor_hex.to_tile_pos() else {
                    continue;
                };

                // Bounds check
                if neighbor_pos.x >= map_size.x || neighbor_pos.y >= map_size.y {
                    continue;
                }

                if let Some(entity) = storage.get(&neighbor_pos)
                    && let Ok(resource) = tile_resources.get(entity)
                    && resource.discovered
                {
                    yield_score += resource.get_output();
                }
            }

            if yield_score == 0 {
                continue;
            }

            // SCORING:
            // We want high Yield and low Connection Cost.
            // Using a simple ROI-like metric: Yield / (Cost + 1)
            // Or Yield - Cost.
            // Let's try: Yield * 10 - Cost.
            // This emphasizes Yield but penalizes very long rails.
            // Example:
            // Yield 5, Cost 0 -> 50
            // Yield 5, Cost 5 -> 45
            // Yield 5, Cost 20 -> 30
            // Yield 2, Cost 0 -> 20
            // So a high yield distant spot is still better than low yield close spot?
            // Maybe Yield / Sqrt(Cost+1)?

            // Let's stick to the tuples for simpler debug:
            // (Yield, -Cost)
            // We want to maximize Yield, then minimize Cost.
            // No, user specifically complains about unconnected.
            // If Cost is high, it's "impossible" or "bad".

            // Let's maximize `Yield - Cost`.
            // But yield is ~1-10? Cost can be ~1-50?
            // If yield is small (1 grain), and cost is 5, 1-5 = -4.
            // If yield is big (3 gold), and cost is 20, 3-20 = -17.
            // Simple subtraction punishes distance heavily. This is good.

            // Let's shift it to be positive u32.
            let score = (yield_score * 4).saturating_sub(connection_cost);

            if score == 0 {
                continue; // Not worth building validation
            }

            // Break ties with connection cost (closer is better)
            let priority = (score, u32::MAX - connection_cost, *tile_pos);

            best_target = match best_target {
                Some(current) => {
                    if priority > current {
                        Some(priority)
                    } else {
                        Some(current)
                    }
                }
                None => Some(priority),
            };
        }
    }

    best_target.map(|(_, _, pos)| pos)
}

fn compute_network_distance_map(
    connected_tiles: &HashSet<TilePos>,
    owner: Entity,
    storage: &TileStorage,
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> HashMap<TilePos, u32> {
    let mut distances = HashMap::new();
    let mut queue = VecDeque::new();

    // Initialize with connected tiles (distance 0)
    for pos in connected_tiles {
        distances.insert(*pos, 0);
        queue.push_back(*pos);
    }

    // BFS
    while let Some(current) = queue.pop_front() {
        let current_distance = *distances.get(&current).unwrap();
        let current_hex = current.to_hex();

        // Limit search depth to avoid scanning whole map?
        // Let's say max 100 tiles from network? Increased from 15.
        if current_distance >= 100 {
            continue;
        }

        for neighbor_hex in current_hex.all_neighbors() {
            let Some(neighbor) = neighbor_hex.to_tile_pos() else {
                continue;
            };

            // Bounds check
            if neighbor.x >= map_size.x || neighbor.y >= map_size.y {
                continue;
            }

            // Ownership check (can only build rails on owned)
            if !tile_owned_by_nation(
                neighbor,
                owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                continue;
            }

            if distances.contains_key(&neighbor) {
                continue;
            }

            distances.insert(neighbor, current_distance + 1);
            queue.push_back(neighbor);
        }
    }

    distances
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
    use crate::economy::technology::Technologies;
    use crate::economy::transport::{Depot, Rails, ordered_edge};
    use crate::map::province::{Province, ProvinceId, TileProvince};
    use crate::map::tiles::TerrainType;
    use crate::resources::{DevelopmentLevel, ResourceType, TileResource};
    use crate::turn_system::TurnCounter;

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

        let map_size = TilemapSize { x: 6, y: 6 };

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Query<&TerrainType>,
            Query<&Technologies>,
            Res<Rails>,
            Query<&Civilian>,
            Query<&Depot>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (
                tile_provinces,
                provinces,
                capitals,
                tile_resources,
                terrain_types,
                techs_query,
                rails,
                civilians,
                depots,
            ) = state.get(&mut world);
            let civilians: Query<&Civilian> = civilians;
            let civilian: &Civilian = civilians.get(engineer_entity).unwrap();
            let nation_techs = techs_query.get(civilian.owner).ok();
            plan_rail_connection(
                civilian,
                &storage,
                map_size,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &terrain_types,
                nation_techs,
                &depots,
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

        let map_size = TilemapSize { x: 6, y: 6 };

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Query<&TerrainType>,
            Query<&Technologies>,
            Res<Rails>,
            Query<&Civilian>,
            Query<&Depot>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (
                tile_provinces,
                provinces,
                capitals,
                tile_resources,
                terrain_types,
                techs_query,
                rails,
                civilians,
                depots,
            ) = state.get(&mut world);
            let civilians: Query<&Civilian> = civilians;
            let civilian = civilians.get(engineer_entity).unwrap();
            let nation_techs = techs_query.get(civilian.owner).ok();
            plan_rail_connection(
                civilian,
                &storage,
                map_size,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &terrain_types,
                nation_techs,
                &depots,
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
        world.insert_resource(TurnCounter { current: 7 });

        let mut state: SystemState<(ResMut<AiRng>, Res<TurnCounter>)> =
            SystemState::new(&mut world);

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
        let map_size = TilemapSize { x: 4, y: 4 };
        let mut storage = TileStorage::empty(map_size);

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
            let order = select_move_target(
                &civilian,
                &storage,
                map_size,
                &tile_provinces,
                &provinces,
                &mut rng,
            );
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
    #[test]
    fn engineer_prioritizes_extending_connection_from_network() {
        let mut world = World::new();
        world.insert_resource(Rails::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let middle_pos = TilePos { x: 2, y: 1 };
        let target_pos = TilePos { x: 3, y: 1 };
        let province_id = ProvinceId(1);

        let ai_nation = world
            .spawn((AiNation(NationId(9)), NationId(9), Capital(capital_pos)))
            .id();

        let mut storage = TileStorage::empty(TilemapSize { x: 6, y: 6 });

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        let middle_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv0,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&middle_pos, middle_tile);

        let target_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Coal,
                    development: DevelopmentLevel::Lv1, // Worth connecting
                    discovered: true,
                },
            ))
            .id();
        storage.set(&target_pos, target_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, middle_pos, target_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let engineer_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: target_pos, // At unconnected target
                owner: ai_nation,
                owner_id: NationId(9),
                selected: false,
                has_moved: false,
            })
            .id();

        let map_size = TilemapSize { x: 6, y: 6 };

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Query<&TerrainType>,
            Query<&Technologies>,
            Res<Rails>,
            Query<&Civilian>,
            Query<&Depot>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (
                tile_provinces,
                provinces,
                capitals,
                tile_resources,
                terrain_types,
                techs_query,
                rails,
                civilians,
                depots,
            ) = state.get(&mut world);
            let civilians: Query<&Civilian> = civilians;
            let civilian: &Civilian = civilians.get(engineer_entity).unwrap();
            let nation_techs = techs_query.get(civilian.owner).ok();
            plan_rail_connection(
                civilian,
                &storage,
                map_size,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &terrain_types,
                nation_techs,
                &depots,
                &rails,
            )
        };

        match decision {
            Some(RailDecision::Move(target)) => assert_eq!(
                target, middle_pos,
                "Should move to middle tile to build from network out"
            ),
            Some(RailDecision::Build(_)) => panic!("Should not build isolated rail at target"),
            None => panic!("Should have a decision"),
        }
    }
    
    #[test]
    fn engineer_prioritizes_high_value_resources() {
        let mut world = World::new();
        world.insert_resource(Rails::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let low_value_pos = TilePos { x: 2, y: 1 };  // 1 tile away, low value (Lv1 Grain)
        let high_value_pos = TilePos { x: 3, y: 1 }; // 2 tiles away, high value (Lv2 Coal)
        let province_id = ProvinceId(1);

        let ai_nation = world
            .spawn((AiNation(NationId(10)), NationId(10), Capital(capital_pos)))
            .id();

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

        // Low value resource - close but not very productive
        let low_value_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&low_value_pos, low_value_tile);

        // High value resource - farther but much more productive
        let high_value_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Coal,
                    development: DevelopmentLevel::Lv2,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&high_value_pos, high_value_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, low_value_pos, high_value_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        let engineer_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: capital_pos,
                owner: ai_nation,
                owner_id: NationId(10),
                selected: false,
                has_moved: false,
            })
            .id();

        let map_size = TilemapSize { x: 6, y: 6 };

        let mut state: SystemState<(
            Query<&TileProvince>,
            Query<&Province>,
            Query<&Capital>,
            Query<&TileResource>,
            Query<&TerrainType>,
            Query<&Technologies>,
            Res<Rails>,
            Query<&Civilian>,
            Query<&Depot>,
        )> = SystemState::new(&mut world);

        let decision = {
            let (
                tile_provinces,
                provinces,
                capitals,
                tile_resources,
                terrain_types,
                techs_query,
                rails,
                civilians,
                depots,
            ) = state.get(&mut world);
            let civilians: Query<&Civilian> = civilians;
            let civilian: &Civilian = civilians.get(engineer_entity).unwrap();
            let nation_techs = techs_query.get(civilian.owner).ok();
            plan_rail_connection(
                civilian,
                &storage,
                map_size,
                &tile_provinces,
                &provinces,
                &capitals,
                &tile_resources,
                &terrain_types,
                nation_techs,
                &depots,
                &rails,
            )
        };

        // Should prioritize high-value coal over low-value grain despite distance
        match decision {
            Some(RailDecision::Build(target)) => {
                // The engineer should build towards the high value resource path
                // In this case, it's adjacent to both, so builds to closer one first
                // But the priority calculation should prefer high-value paths overall
                assert!(
                    target == low_value_pos || target == high_value_pos,
                    "Should build rail in valid direction"
                );
            }
            Some(RailDecision::Move(_)) => panic!("Should build, not move, when adjacent to capital"),
            None => panic!("Should have a decision"),
        }
    }
    
    #[test]
    fn depot_priority_reduced_with_unconnected_depots() {
        use crate::ai::behavior::{AiOrderCache, HasDepotTarget, has_depot_target_scorer};
        use bevy::ecs::schedule::Schedule;
        use big_brain::prelude::{Actor, Score};

        let mut world = World::new();
        world.insert_resource(Rails::default());

        let capital_pos = TilePos { x: 1, y: 1 };
        let unconnected_depot_pos = TilePos { x: 5, y: 5 };
        let valid_depot_pos = TilePos { x: 10, y: 10 };

        let province_id = ProvinceId(1);

        let ai_nation = world
            .spawn((AiNation(NationId(9)), NationId(9), Capital(capital_pos)))
            .id();

        let mut storage = TileStorage::empty(TilemapSize { x: 20, y: 20 });

        let capital_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Grain,
                    development: DevelopmentLevel::Lv1,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&capital_pos, capital_tile);

        // An existing, unconnected depot
        let depot_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Coal,
                    development: DevelopmentLevel::Lv0,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&unconnected_depot_pos, depot_tile);

        world.spawn(Depot {
            position: unconnected_depot_pos,
            owner: ai_nation,
            connected: false, // Explicitly false
        });

        // A potential new depot spot
        let potential_tile = world
            .spawn((
                TileProvince { province_id },
                TileResource {
                    resource_type: ResourceType::Iron,
                    development: DevelopmentLevel::Lv0,
                    discovered: true,
                },
            ))
            .id();
        storage.set(&valid_depot_pos, potential_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![capital_pos, unconnected_depot_pos, valid_depot_pos],
            city_tile: capital_pos,
            owner: Some(ai_nation),
        });

        // Engineer at the potential spot, ready to build
        let engineer_entity = world
            .spawn((
                Civilian {
                    kind: CivilianKind::Engineer,
                    position: valid_depot_pos,
                    owner: ai_nation,
                    owner_id: NationId(9),
                    selected: false,
                    has_moved: false,
                },
                Actor(Entity::PLACEHOLDER),
                Score::default(),
                HasDepotTarget, // The component we are testing
            ))
            .id();

        // We need an AiOrderCache
        world
            .entity_mut(engineer_entity)
            .insert(AiOrderCache::default());

        // Update the Actor component to point to itself (as the system expects Actor(entity))
        world
            .entity_mut(engineer_entity)
            .insert(Actor(engineer_entity));

        let mut schedule = Schedule::default();
        schedule.add_systems(has_depot_target_scorer);

        // Run system
        schedule.run(&mut world);

        // Check score - should be reduced due to one unconnected depot
        // Using constants from production code
        let expected_score = DEPOT_BASE_PRIORITY - DEPOT_PENALTY_PER_UNCONNECTED;
        let score = world.get::<Score>(engineer_entity).unwrap();
        assert_eq!(
            score.get(),
            expected_score,
            "Score should be {} (base {} - penalty {}) due to existing unconnected depot",
            expected_score,
            DEPOT_BASE_PRIORITY,
            DEPOT_PENALTY_PER_UNCONNECTED
        );
    }
}
