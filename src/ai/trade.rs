use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::ai::context::enemy_turn_entered;
use crate::ai::markers::AiNation;
use crate::civilians::Civilian;
use crate::civilians::CivilianKind;
use crate::economy::allocation_systems;
use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::{Allocations, EconomySet, NationHandle, NationId, Stockpile, Treasury};
use crate::messages::{AdjustMarketOrder, HireCivilian, MarketInterest};
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

/// Registers simple economic behaviours for AI nations such as placing
/// market buy/sell orders based on stockpile levels.
pub struct AiEconomyPlugin;

impl Plugin for AiEconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                plan_ai_market_orders
                    .in_set(EconomySet)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_entered)
                    .before(allocation_systems::apply_market_order_adjustments),
                plan_ai_civilian_hiring
                    .in_set(EconomySet)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_entered),
            ),
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

fn plan_ai_market_orders(
    mut writer: MessageWriter<AdjustMarketOrder>,
    ai_nations: Query<
        (
            &NationHandle,
            &NationId,
            &Allocations,
            &Stockpile,
            &Treasury,
        ),
        With<AiNation>,
    >,
    pricing: Res<MarketPriceModel>,
) {
    for (handle, nation_id, allocations, stockpile, treasury) in ai_nations.iter() {
        let nation = handle.instance();
        let cash_available = treasury.available();

        for &good in MARKET_RESOURCES {
            let available = stockpile.get_available(good);
            let price = pricing.price_for(good, MarketVolume::default()) as i64;
            let has_buy_interest = allocations.has_buy_interest(good);
            let wants_buy = available <= BUY_SHORTAGE_THRESHOLD;
            let can_afford = cash_available >= price && price > 0;

            // Express buy interest (boolean) if we have a shortage and can afford it
            if wants_buy && can_afford {
                if !has_buy_interest {
                    info!(
                        "AI Nation {:?}: Expressing buy interest for {:?} (available: {}, price: ${})",
                        nation_id, good, available, price
                    );
                    writer.write(AdjustMarketOrder {
                        nation,
                        good,
                        kind: MarketInterest::Buy,
                        requested: 1, // Non-zero means "interested"
                    });
                }
            } else if has_buy_interest {
                // Clear buy interest if we no longer want/can afford it
                info!(
                    "AI Nation {:?}: Clearing buy interest for {:?} (available: {}, can_afford: {})",
                    nation_id, good, available, can_afford
                );
                writer.write(AdjustMarketOrder {
                    nation,
                    good,
                    kind: MarketInterest::Buy,
                    requested: 0, // Zero means "not interested"
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
                    "AI Nation {:?}: Adjusting sell orders for {:?} from {} to {} (available: {})",
                    nation_id, good, current_sell, desired_sell, available
                );
                writer.write(AdjustMarketOrder {
                    nation,
                    good,
                    kind: MarketInterest::Sell,
                    requested: desired_sell,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::MessageReader;
    use bevy::ecs::system::{RunSystemOnce, SystemState};
    use bevy::prelude::{App, Entity, World};
    use bevy_ecs_tilemap::prelude::TilePos;

    use crate::ai::markers::AiNation;
    use crate::ai::trade::{
        AI_CIVILIAN_TARGETS, SELL_MAX_PER_GOOD, SELL_RESERVE, plan_ai_civilian_hiring,
        plan_ai_market_orders,
    };
    use crate::civilians::Civilian;
    use crate::economy::{
        allocation::Allocations,
        goods::Good,
        market::MarketPriceModel,
        nation::{NationHandle, NationId, NationInstance},
        stockpile::Stockpile,
        treasury::Treasury,
    };
    use crate::messages::{AdjustMarketOrder, HireCivilian, MarketInterest};

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

    fn drain_orders(world: &mut World) -> Vec<AdjustMarketOrder> {
        let mut state: SystemState<MessageReader<AdjustMarketOrder>> = SystemState::new(world);
        let mut reader = state.get_mut(world);
        let mut orders = Vec::new();
        for msg in reader.read() {
            orders.push(*msg);
        }
        state.apply(world);
        orders
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
    fn issues_buy_interest_for_shortages() {
        let mut app = App::new();
        app.add_message::<AdjustMarketOrder>();
        app.add_message::<HireCivilian>();
        app.insert_resource(MarketPriceModel::default());

        let nation = spawn_ai_nation(&mut app);
        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(nation)
                .unwrap()
                .add(Good::Grain, 1);
        }

        let _ = app.world_mut().run_system_once(plan_ai_market_orders);
        let orders = drain_orders(app.world_mut());

        let grain_order = orders
            .iter()
            .find(|order| order.kind == MarketInterest::Buy && order.good == Good::Grain)
            .expect("expected grain buy order");
        // Buy interest is boolean - just check that interest was expressed
        assert!(grain_order.requested > 0, "Expected buy interest for Grain");
    }

    #[test]
    fn issues_sell_orders_for_surplus() {
        let mut app = App::new();
        app.add_message::<AdjustMarketOrder>();
        app.add_message::<HireCivilian>();
        app.insert_resource(MarketPriceModel::default());

        let nation = spawn_ai_nation(&mut app);
        {
            let world = app.world_mut();
            let mut stockpile = world.get_mut::<Stockpile>(nation).unwrap();
            stockpile.add(Good::Coal, SELL_RESERVE + SELL_MAX_PER_GOOD + 2);
        }

        let _ = app.world_mut().run_system_once(plan_ai_market_orders);
        let orders = drain_orders(app.world_mut());

        assert!(orders.iter().any(|order| {
            order.kind == MarketInterest::Sell
                && order.good == Good::Coal
                && order.requested == SELL_MAX_PER_GOOD
        }));
    }

    #[test]
    fn clears_buy_interest_when_broke() {
        let mut app = App::new();
        app.add_message::<AdjustMarketOrder>();
        app.add_message::<HireCivilian>();
        app.insert_resource(MarketPriceModel::default());

        let nation = spawn_ai_nation(&mut app);
        {
            let world = app.world_mut();
            world.get_mut::<Treasury>(nation).unwrap().subtract(1_000);
            let mut allocations = world.get_mut::<Allocations>(nation).unwrap();
            allocations.market_buys.insert(Good::Fish);
        }

        let _ = app.world_mut().run_system_once(plan_ai_market_orders);
        let orders = drain_orders(app.world_mut());

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
