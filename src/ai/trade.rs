use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::ai::context::enemy_turn_entered;
use crate::ai::markers::AiNation;
use crate::economy::allocation_systems;
use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::{Allocations, EconomySet, NationHandle, Stockpile, Treasury};
use crate::messages::{AdjustMarketOrder, MarketInterest};
use crate::ui::menu::AppState;

const BUY_SHORTAGE_THRESHOLD: u32 = 2;
const SELL_RESERVE: u32 = 5;
const SELL_MAX_PER_GOOD: u32 = 6;

/// Registers simple economic behaviours for AI nations such as placing
/// market buy/sell orders based on stockpile levels.
pub struct AiEconomyPlugin;

impl Plugin for AiEconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            plan_ai_market_orders
                .in_set(EconomySet)
                .run_if(in_state(AppState::InGame))
                .run_if(enemy_turn_entered)
                .before(allocation_systems::apply_market_order_adjustments),
        );
    }
}

fn plan_ai_market_orders(
    mut writer: MessageWriter<AdjustMarketOrder>,
    ai_nations: Query<(&NationHandle, &Allocations, &Stockpile, &Treasury), With<AiNation>>,
    pricing: Res<MarketPriceModel>,
) {
    for (handle, allocations, stockpile, treasury) in ai_nations.iter() {
        let nation = handle.instance();
        let cash_available = treasury.available();

        for &good in MARKET_RESOURCES {
            let available = stockpile.get_available(good);
            let price = pricing.price_for(good, MarketVolume::default()) as i64;
            let current_buy = allocations.market_buy_quantity(good);
            let wants_buy = available <= BUY_SHORTAGE_THRESHOLD;
            let can_afford = cash_available >= price && price > 0;

            if wants_buy && can_afford {
                let deficit = BUY_SHORTAGE_THRESHOLD
                    .saturating_add(1)
                    .saturating_sub(available);
                let desired_buy = deficit.max(1) as u32;

                if current_buy != desired_buy {
                    writer.write(AdjustMarketOrder {
                        nation,
                        good,
                        kind: MarketInterest::Buy,
                        requested: desired_buy,
                    });
                }
            } else if current_buy > 0 {
                writer.write(AdjustMarketOrder {
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

    use crate::ai::markers::AiNation;
    use crate::ai::trade::{
        BUY_SHORTAGE_THRESHOLD, SELL_MAX_PER_GOOD, SELL_RESERVE, plan_ai_market_orders,
    };
    use crate::economy::{
        allocation::Allocations,
        goods::Good,
        market::MarketPriceModel,
        nation::{NationHandle, NationId, NationInstance},
        stockpile::Stockpile,
        treasury::Treasury,
    };
    use crate::messages::{AdjustMarketOrder, MarketInterest};

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

    #[test]
    fn issues_buy_interest_for_shortages() {
        let mut app = App::new();
        app.add_message::<AdjustMarketOrder>();
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
        let available = 1;
        let expected = (BUY_SHORTAGE_THRESHOLD + 1)
            .saturating_sub(available)
            .max(1);
        assert_eq!(grain_order.requested, expected);
    }

    #[test]
    fn issues_sell_orders_for_surplus() {
        let mut app = App::new();
        app.add_message::<AdjustMarketOrder>();
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
        app.insert_resource(MarketPriceModel::default());

        let nation = spawn_ai_nation(&mut app);
        {
            let world = app.world_mut();
            world.get_mut::<Treasury>(nation).unwrap().subtract(1_000);
            let mut allocations = world.get_mut::<Allocations>(nation).unwrap();
            allocations.market_buys.insert(Good::Fish, 2);
        }

        let _ = app.world_mut().run_system_once(plan_ai_market_orders);
        let orders = drain_orders(app.world_mut());

        assert!(orders.iter().any(|order| {
            order.kind == MarketInterest::Buy && order.good == Good::Fish && order.requested == 0
        }));
    }
}
