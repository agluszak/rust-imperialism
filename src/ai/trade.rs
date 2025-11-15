use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use big_brain::prelude::*;
use std::collections::HashMap;

use crate::ai::markers::AiNation;
use crate::civilians::Civilian;
use crate::civilians::CivilianKind;
use crate::economy::allocation_systems;
use crate::economy::goods::Good;
use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::production::{BuildingKind, Buildings};
use crate::economy::{Allocations, EconomySet, NationHandle, NationInstance, Stockpile, Treasury};
use crate::messages::{AdjustMarketOrder, AdjustProduction, HireCivilian, MarketInterest};
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::menu::AppState;

const BUY_SHORTAGE_THRESHOLD: u32 = 2;
const SELL_RESERVE: u32 = 5;
const SELL_MAX_PER_GOOD: u32 = 6;
const AI_CIVILIAN_MAX_HIRES_PER_TURN: usize = 1;
const AI_CIVILIAN_TARGETS: &[(CivilianKind, u32)] = &[
    (CivilianKind::Engineer, 2),
    (CivilianKind::Prospector, 2),
    (CivilianKind::Farmer, 2),
    (CivilianKind::Miner, 2),
    (CivilianKind::Rancher, 1),
    (CivilianKind::Forester, 1),
];

const PRODUCTION_PRIORITIES: &[(Good, u32)] = &[
    (Good::CannedFood, 8),
    (Good::Clothing, 6),
    (Good::Furniture, 6),
    (Good::Steel, 4),
    (Good::Fabric, 6),
];

/// Registers economic behaviours for AI nations, including building plans,
/// production allocations, and market participation driven by Big Brain.
pub struct AiEconomyPlugin;

impl Plugin for AiEconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            initialize_ai_economy_thinkers
                .run_if(in_state(AppState::InGame))
                .in_set(EconomySet),
        )
        .add_systems(
            Update,
            (
                plan_buildings_scorer,
                apply_production_scorer,
                plan_market_scorer,
                economy_idle_scorer,
            )
                .in_set(BigBrainSet::Scorers)
                .run_if(in_state(AppState::InGame))
                .run_if(enemy_turn_active)
                .in_set(EconomySet),
        )
        .add_systems(
            Update,
            (
                plan_building_focus_action,
                apply_production_plan_action,
                plan_market_orders_action,
                idle_economy_action,
            )
                .in_set(BigBrainSet::Actions)
                .run_if(in_state(AppState::InGame))
                .run_if(enemy_turn_active)
                .before(allocation_systems::apply_production_adjustments)
                .before(allocation_systems::apply_market_order_adjustments)
                .in_set(EconomySet),
        )
        .add_systems(
            Update,
            plan_ai_civilian_hiring
                .in_set(EconomySet)
                .run_if(in_state(AppState::InGame))
                .run_if(enemy_turn_active),
        );
    }
}

fn plan_ai_civilian_hiring(
    mut writer: MessageWriter<HireCivilian>,
    ai_nations: Query<(&NationHandle, &Treasury), With<AiNation>>,
    civilians: Query<&Civilian>,
) {
    let mut counts: HashMap<Entity, HashMap<CivilianKind, u32>> = HashMap::new();
    for civilian in civilians.iter() {
        counts
            .entry(civilian.owner)
            .or_default()
            .entry(civilian.kind)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    for (handle, treasury) in ai_nations.iter() {
        let nation = handle.instance();
        let mut remaining_cash = treasury.available();
        let mut hires_this_turn = 0;
        let nation_counts = counts.get(&nation.entity());

        for &(kind, target) in AI_CIVILIAN_TARGETS {
            if hires_this_turn >= AI_CIVILIAN_MAX_HIRES_PER_TURN {
                break;
            }

            let current = nation_counts
                .and_then(|entries| entries.get(&kind))
                .copied()
                .unwrap_or(0);
            if current >= target {
                continue;
            }

            let cost = kind.hiring_cost();
            if remaining_cash < cost {
                break;
            }

            writer.write(HireCivilian { nation, kind });
            remaining_cash -= cost;
            hires_this_turn += 1;
        }
    }
}

#[derive(Component, Debug, Default)]
struct AiEconomyBrain {
    planned_production: Vec<AdjustProduction>,
    last_building_turn: Option<u32>,
    last_production_turn: Option<u32>,
    last_market_turn: Option<u32>,
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
struct ShouldPlanBuildings;

#[derive(Component, Debug, Clone, Default, ActionBuilder)]
struct PlanBuildingFocus;

#[derive(Component, Debug, Clone, ScorerBuilder)]
struct ShouldApplyProduction;

#[derive(Component, Debug, Clone, Default, ActionBuilder)]
struct ApplyProductionPlan;

#[derive(Component, Debug, Clone, ScorerBuilder)]
struct ShouldPlanMarket;

#[derive(Component, Debug, Clone, Default, ActionBuilder)]
struct PlanMarketOrders;

#[derive(Component, Debug, Clone, ScorerBuilder)]
struct EconomyIdle;

#[derive(Component, Debug, Clone, Default, ActionBuilder)]
struct EconomyIdleAction;

fn initialize_ai_economy_thinkers(
    mut commands: Commands,
    uninitialized: Query<Entity, (With<AiNation>, Without<AiEconomyBrain>)>,
) {
    for entity in &uninitialized {
        commands.entity(entity).insert((
            AiEconomyBrain::default(),
            Thinker::build()
                .label("ai_economy")
                .picker(FirstToScore { threshold: 0.5 })
                .when(ShouldPlanBuildings, PlanBuildingFocus)
                .when(ShouldApplyProduction, ApplyProductionPlan)
                .when(ShouldPlanMarket, PlanMarketOrders)
                .when(EconomyIdle, EconomyIdleAction),
        ));
    }
}

fn plan_buildings_scorer(
    turn: Res<TurnSystem>,
    mut scores: Query<
        (&Actor, &mut Score, &AiEconomyBrain, &ScorerSpan),
        With<ShouldPlanBuildings>,
    >,
) {
    for (_, mut score, brain, span) in &mut scores {
        let ready = brain.last_building_turn != Some(turn.current_turn);
        span.span().in_scope(|| {
            trace!(
                "AI economy building score: {}",
                if ready { 0.95 } else { 0.0 }
            );
        });
        score.set(if ready { 0.95 } else { 0.0 });
    }
}

fn apply_production_scorer(
    turn: Res<TurnSystem>,
    mut scores: Query<
        (&Actor, &mut Score, &AiEconomyBrain, &ScorerSpan),
        With<ShouldApplyProduction>,
    >,
) {
    for (_, mut score, brain, span) in &mut scores {
        let ready = brain.last_production_turn != Some(turn.current_turn)
            && !brain.planned_production.is_empty();
        span.span().in_scope(|| {
            trace!(
                "AI economy production score: {}",
                if ready { 0.9 } else { 0.0 }
            );
        });
        score.set(if ready { 0.9 } else { 0.0 });
    }
}

fn plan_market_scorer(
    turn: Res<TurnSystem>,
    mut scores: Query<(&Actor, &mut Score, &AiEconomyBrain, &ScorerSpan), With<ShouldPlanMarket>>,
) {
    for (_, mut score, brain, span) in &mut scores {
        let ready = brain.last_market_turn != Some(turn.current_turn);
        span.span().in_scope(|| {
            trace!("AI economy market score: {}", if ready { 0.8 } else { 0.0 });
        });
        score.set(if ready { 0.8 } else { 0.0 });
    }
}

fn economy_idle_scorer(mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<EconomyIdle>>) {
    for (_, mut score, span) in &mut scores {
        span.span()
            .in_scope(|| trace!("AI economy idle scorer active"));
        score.set(0.1);
    }
}

fn plan_building_focus_action(
    turn: Res<TurnSystem>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiEconomyBrain, &ActionSpan),
        With<PlanBuildingFocus>,
    >,
    nations: Query<(&NationHandle, &Buildings, &Stockpile, &Allocations), With<AiNation>>,
) {
    for (Actor(actor), mut state, mut brain, span) in &mut actions {
        if *state != ActionState::Requested {
            continue;
        }

        let Ok((handle, buildings, stockpile, allocations)) = nations.get(*actor) else {
            *state = ActionState::Failure;
            continue;
        };

        let plans =
            evaluate_production_plan(*actor, handle.instance(), buildings, stockpile, allocations);

        brain.planned_production.clear();
        brain.planned_production.extend(plans.into_iter());
        brain.last_building_turn = Some(turn.current_turn);

        span.span().in_scope(|| {
            trace!(
                "AI Nation {:?}: planned {} production adjustments",
                actor,
                brain.planned_production.len()
            );
        });

        *state = ActionState::Success;
    }
}

fn apply_production_plan_action(
    mut writer: MessageWriter<AdjustProduction>,
    turn: Res<TurnSystem>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiEconomyBrain, &ActionSpan),
        With<ApplyProductionPlan>,
    >,
) {
    for (_, mut state, mut brain, span) in &mut actions {
        if *state != ActionState::Requested {
            continue;
        }

        if brain.planned_production.is_empty() {
            *state = ActionState::Success;
            continue;
        }

        for order in brain.planned_production.drain(..) {
            writer.write(order);
        }

        brain.last_production_turn = Some(turn.current_turn);
        span.span()
            .in_scope(|| trace!("AI economy applied production plan"));
        *state = ActionState::Success;
    }
}

fn plan_market_orders_action(
    mut writer: MessageWriter<AdjustMarketOrder>,
    pricing: Res<MarketPriceModel>,
    turn: Res<TurnSystem>,
    mut actions: Query<
        (&Actor, &mut ActionState, &mut AiEconomyBrain, &ActionSpan),
        With<PlanMarketOrders>,
    >,
    nations: Query<(&NationHandle, &Allocations, &Stockpile, &Treasury), With<AiNation>>,
) {
    for (Actor(actor), mut state, mut brain, span) in &mut actions {
        if *state != ActionState::Requested {
            continue;
        }

        let Ok((handle, allocations, stockpile, treasury)) = nations.get(*actor) else {
            *state = ActionState::Failure;
            continue;
        };

        let orders = evaluate_market_orders(
            handle.instance(),
            allocations,
            stockpile,
            treasury,
            &pricing,
        );

        for order in orders.iter().copied() {
            writer.write(order);
        }

        brain.last_market_turn = Some(turn.current_turn);
        span.span().in_scope(|| {
            trace!(
                "AI Nation {:?}: queued {} market orders",
                actor,
                orders.len()
            );
        });

        *state = ActionState::Success;
    }
}

fn idle_economy_action(
    mut actions: Query<(&Actor, &mut ActionState, &ActionSpan), With<EconomyIdleAction>>,
) {
    for (_, mut state, span) in &mut actions {
        if *state != ActionState::Requested {
            continue;
        }
        span.span().in_scope(|| trace!("AI economy idle action"));
        *state = ActionState::Success;
    }
}

fn enemy_turn_active(turn: Res<TurnSystem>) -> bool {
    turn.phase == TurnPhase::EnemyTurn
}

fn building_for_good(good: Good) -> Option<BuildingKind> {
    match good {
        Good::Fabric => Some(BuildingKind::TextileMill),
        Good::Paper | Good::Lumber => Some(BuildingKind::LumberMill),
        Good::Steel => Some(BuildingKind::SteelMill),
        Good::CannedFood => Some(BuildingKind::FoodProcessingCenter),
        Good::Clothing => Some(BuildingKind::ClothingFactory),
        Good::Furniture => Some(BuildingKind::FurnitureFactory),
        Good::Hardware | Good::Armaments => Some(BuildingKind::MetalWorks),
        Good::Fuel => Some(BuildingKind::Refinery),
        Good::Transport => Some(BuildingKind::Railyard),
        _ => None,
    }
}

fn evaluate_production_plan(
    nation_entity: Entity,
    nation: NationInstance,
    buildings: &Buildings,
    stockpile: &Stockpile,
    allocations: &Allocations,
) -> Vec<AdjustProduction> {
    let mut plans = Vec::new();

    for &(good, desired_stock) in PRODUCTION_PRIORITIES {
        let Some(kind) = building_for_good(good) else {
            continue;
        };
        let Some(building) = buildings.get(kind) else {
            continue;
        };

        let available = stockpile.get_available(good);
        let current = allocations.production_count(nation_entity, good) as u32;
        let shortage = desired_stock.saturating_sub(available);
        let target = shortage.min(building.capacity);

        if target == current {
            continue;
        }

        if target == 0 && current == 0 {
            continue;
        }

        plans.push(AdjustProduction {
            nation,
            building: nation_entity,
            output_good: good,
            target_output: target,
        });
    }

    plans
}

fn evaluate_market_orders(
    nation: NationInstance,
    allocations: &Allocations,
    stockpile: &Stockpile,
    treasury: &Treasury,
    pricing: &MarketPriceModel,
) -> Vec<AdjustMarketOrder> {
    let mut orders = Vec::new();
    let cash_available = treasury.available();

    for &good in MARKET_RESOURCES {
        let available = stockpile.get_available(good);
        let price = pricing.price_for(good, MarketVolume::default()) as i64;
        let has_buy_interest = allocations.has_buy_interest(good);
        let wants_buy = available <= BUY_SHORTAGE_THRESHOLD;
        let can_afford = cash_available >= price && price > 0;

        if wants_buy && can_afford {
            if !has_buy_interest {
                info!(
                    "AI Nation {:?}: expressing buy interest for {:?} (available: {}, price: ${})",
                    nation.entity(),
                    good,
                    available,
                    price
                );
                orders.push(AdjustMarketOrder {
                    nation,
                    good,
                    kind: MarketInterest::Buy,
                    requested: 1,
                });
            }
        } else if has_buy_interest {
            info!(
                "AI Nation {:?}: clearing buy interest for {:?}",
                nation.entity(),
                good
            );
            orders.push(AdjustMarketOrder {
                nation,
                good,
                kind: MarketInterest::Buy,
                requested: 0,
            });
        }

        let desired_sell = if available > SELL_RESERVE {
            (available - SELL_RESERVE).min(SELL_MAX_PER_GOOD)
        } else {
            0
        };
        let current_sell = allocations.market_sell_count(good) as u32;

        if desired_sell != current_sell {
            info!(
                "AI Nation {:?}: adjusting sell orders for {:?} from {} to {}",
                nation.entity(),
                good,
                current_sell,
                desired_sell
            );
            orders.push(AdjustMarketOrder {
                nation,
                good,
                kind: MarketInterest::Sell,
                requested: desired_sell,
            });
        }
    }

    orders
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::MessageReader;
    use bevy::ecs::system::{RunSystemOnce, SystemState};
    use bevy::prelude::{App, Entity, World};
    use bevy_ecs_tilemap::prelude::TilePos;

    use crate::ai::markers::AiNation;
    use crate::ai::trade::{
        AI_CIVILIAN_TARGETS, SELL_MAX_PER_GOOD, SELL_RESERVE, plan_ai_civilian_hiring,
    };
    use crate::civilians::Civilian;
    use crate::economy::goods::Good;
    use crate::economy::market::MarketPriceModel;
    use crate::economy::nation::{NationHandle, NationId, NationInstance};
    use crate::economy::production::Buildings;
    use crate::economy::stockpile::Stockpile;
    use crate::economy::treasury::Treasury;
    use crate::messages::{HireCivilian, MarketInterest};

    fn nation_instance(world: &World, entity: Entity) -> NationInstance {
        NationInstance::from_entity(world.entity(entity)).unwrap()
    }

    fn spawn_ai_nation(app: &mut App) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                AiNation,
                NationId(42),
                Allocations::default(),
                Stockpile::default(),
                Treasury::new(1_000),
            ))
            .id();

        let world = app.world_mut();
        let instance = NationInstance::from_entity(world.entity(entity)).unwrap();
        world.entity_mut(entity).insert(NationHandle::new(instance));
        entity
    }

    fn drain_hires(world: &mut World) -> Vec<HireCivilian> {
        let mut state: SystemState<MessageReader<HireCivilian>> = SystemState::new(world);
        let mut reader = state.get_mut(world);
        let mut hires = Vec::new();
        for msg in reader.read() {
            hires.push(*msg);
        }
        state.apply(world);
        hires
    }

    #[test]
    fn production_plan_targets_shortages() {
        let mut world = World::new();
        let nation = world.spawn(NationId(7)).id();
        let instance = nation_instance(&world, nation);

        let buildings = Buildings::with_all_initial();
        let stockpile = Stockpile::default();
        let allocations = Allocations::default();

        let plans =
            evaluate_production_plan(nation, instance, &buildings, &stockpile, &allocations);
        assert!(
            plans
                .iter()
                .any(|order| order.output_good == Good::CannedFood && order.target_output > 0)
        );
    }

    #[test]
    fn market_orders_request_buy_when_short() {
        let mut world = World::new();
        let nation = world.spawn(NationId(1)).id();
        let instance = nation_instance(&world, nation);

        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Grain, 1);
        let allocations = Allocations::default();
        let treasury = Treasury::new(1_000);
        let pricing = MarketPriceModel::default();

        let orders =
            evaluate_market_orders(instance, &allocations, &stockpile, &treasury, &pricing);
        assert!(orders.iter().any(|order| {
            order.kind == MarketInterest::Buy && order.good == Good::Grain && order.requested > 0
        }));
    }

    #[test]
    fn market_orders_sell_surplus_goods() {
        let mut world = World::new();
        let nation = world.spawn(NationId(2)).id();
        let instance = nation_instance(&world, nation);

        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Coal, SELL_RESERVE + SELL_MAX_PER_GOOD + 3);
        let allocations = Allocations::default();
        let treasury = Treasury::new(500);
        let pricing = MarketPriceModel::default();

        let orders =
            evaluate_market_orders(instance, &allocations, &stockpile, &treasury, &pricing);
        assert!(orders.iter().any(|order| {
            order.kind == MarketInterest::Sell
                && order.good == Good::Coal
                && order.requested == SELL_MAX_PER_GOOD
        }));
    }

    #[test]
    fn market_orders_clear_buy_interest_when_broke() {
        let mut world = World::new();
        let nation = world.spawn(NationId(3)).id();
        let instance = nation_instance(&world, nation);

        let stockpile = Stockpile::default();
        let mut allocations = Allocations::default();
        allocations.market_buys.insert(Good::Fish);
        let treasury = Treasury::new(0);
        let pricing = MarketPriceModel::default();

        let orders =
            evaluate_market_orders(instance, &allocations, &stockpile, &treasury, &pricing);
        assert!(orders.iter().any(|order| {
            order.kind == MarketInterest::Buy && order.good == Good::Fish && order.requested == 0
        }));
    }

    #[test]
    fn hires_civilian_when_below_target() {
        let mut app = App::new();
        app.add_message::<HireCivilian>();

        let nation = spawn_ai_nation(&mut app);

        let _ = app.world_mut().run_system_once(plan_ai_civilian_hiring);
        let hires = drain_hires(app.world_mut());

        assert!(hires.iter().any(|hire| hire.nation.entity() == nation));
    }

    #[test]
    fn does_not_hire_when_already_has_targets() {
        let mut app = App::new();
        app.add_message::<HireCivilian>();

        let nation = spawn_ai_nation(&mut app);
        {
            let world = app.world_mut();
            for &(kind, target) in AI_CIVILIAN_TARGETS {
                for _ in 0..target {
                    world.spawn(Civilian {
                        kind,
                        position: TilePos { x: 0, y: 0 },
                        owner: nation,
                        owner_id: NationId(42),
                        selected: false,
                        has_moved: false,
                    });
                }
            }
        }

        let _ = app.world_mut().run_system_once(plan_ai_civilian_hiring);
        let hires = drain_hires(app.world_mut());

        assert!(hires.is_empty());
    }
}
