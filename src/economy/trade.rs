use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::nation::{Name, NationId};
use crate::economy::{
    Allocations, Good, ReservationId, ReservationSystem, Stockpile, Treasury, Workforce,
};

#[derive(Debug, Clone)]
struct NationMarketSnapshot {
    entity: Entity,
    name: Option<String>,
    available_cash: i64,
    buy_orders: HashMap<Good, u32>,
    sell_orders: HashMap<Good, Vec<ReservationId>>,
}

#[derive(Debug, Clone, Copy)]
struct PlannedTrade {
    good: Good,
    price: u32,
    seller: Entity,
    buyer: Entity,
    reservation: ReservationId,
}

/// Matches sell reservations against nations with buy interest and transfers goods
/// and cash between their stockpiles and treasuries. Unsold reservations remain
/// in place so they can be released when allocations reset at the start of the next turn.
pub fn resolve_market_orders(
    mut nations: Query<
        (
            &mut Allocations,
            &mut ReservationSystem,
            &mut Stockpile,
            &mut Workforce,
            &mut Treasury,
            Option<&Name>,
        ),
        With<NationId>,
    >,
    nation_entities: Query<Entity, With<NationId>>,
    pricing: Res<MarketPriceModel>,
) {
    let mut snapshots = Vec::new();

    for entity in nation_entities.iter() {
        if let Ok((allocations, _reservations, _stockpile, _workforce, treasury, name)) =
            nations.get_mut(entity)
        {
            let mut buy_orders: HashMap<Good, u32> = HashMap::new();
            for (&good, &quantity) in allocations.market_buys.iter() {
                if quantity > 0 {
                    buy_orders.insert(good, quantity);
                }
            }
            let mut sell_orders: HashMap<Good, Vec<ReservationId>> = HashMap::new();
            for (good, reservations) in allocations.market_sells.iter() {
                if !reservations.is_empty() {
                    sell_orders.insert(*good, reservations.clone());
                }
            }

            snapshots.push(NationMarketSnapshot {
                entity,
                name: name.map(|n| n.0.clone()),
                available_cash: treasury.available(),
                buy_orders,
                sell_orders,
            });
        }
    }

    if snapshots.is_empty() {
        return;
    }

    let mut cash_map: HashMap<Entity, i64> = snapshots
        .iter()
        .map(|snapshot| (snapshot.entity, snapshot.available_cash))
        .collect();
    let mut planned_trades: Vec<PlannedTrade> = Vec::new();

    for &good in MARKET_RESOURCES {
        let mut sellers: Vec<(Entity, Vec<ReservationId>)> = snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot
                    .sell_orders
                    .get(&good)
                    .map(|reservations| (snapshot.entity, reservations.clone()))
            })
            .collect();

        if sellers.is_empty() {
            continue;
        }

        let mut buyers: Vec<(Entity, u32)> = snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot
                    .buy_orders
                    .get(&good)
                    .copied()
                    .filter(|quantity| *quantity > 0)
                    .map(|quantity| (snapshot.entity, quantity))
            })
            .collect();

        if buyers.is_empty() {
            continue;
        }

        let total_supply: u32 = sellers
            .iter()
            .map(|(_, reservations)| reservations.len() as u32)
            .sum();
        let total_demand: u32 = buyers.iter().map(|(_, quantity)| *quantity).sum();
        let volume = MarketVolume::new(total_supply, total_demand);
        let price = pricing.price_for(good, volume) as i64;
        if price <= 0 {
            continue;
        }

        sellers.sort_by_key(|(entity, _)| entity.index());
        buyers.sort_by_key(|(entity, _)| entity.index());

        let mut seller_queue: VecDeque<(Entity, Vec<ReservationId>)> =
            sellers.into_iter().collect();

        'buyers: for (buyer, mut requested) in buyers {
            let Some(mut cash_available) = cash_map.get(&buyer).copied() else {
                continue;
            };

            while requested > 0 {
                if cash_available < price {
                    break;
                }

                if seller_queue.is_empty() {
                    cash_map.insert(buyer, cash_available);
                    break 'buyers;
                }

                let mut seller_entry: Option<(Entity, Vec<ReservationId>)> = None;
                let queue_len = seller_queue.len();
                for _ in 0..queue_len {
                    if let Some((seller_candidate, reservations)) = seller_queue.pop_front() {
                        if seller_candidate == buyer {
                            seller_queue.push_back((seller_candidate, reservations));
                            continue;
                        }
                        seller_entry = Some((seller_candidate, reservations));
                        break;
                    }
                }

                let Some((seller, mut reservations)) = seller_entry else {
                    cash_map.insert(buyer, cash_available);
                    break 'buyers;
                };

                let Some(reservation) = reservations.pop() else {
                    if !reservations.is_empty() {
                        seller_queue.push_back((seller, reservations));
                    }
                    continue;
                };

                planned_trades.push(PlannedTrade {
                    good,
                    price: price as u32,
                    seller,
                    buyer,
                    reservation,
                });

                requested -= 1;
                cash_available -= price;
                *cash_map.entry(seller).or_insert(0) += price;

                if !reservations.is_empty() {
                    seller_queue.push_back((seller, reservations));
                }

                if seller_queue.is_empty() {
                    cash_map.insert(buyer, cash_available);
                    break 'buyers;
                }
            }

            cash_map.insert(buyer, cash_available);
        }
    }

    if planned_trades.is_empty() {
        return;
    }

    let mut name_lookup: HashMap<Entity, Option<String>> = HashMap::new();
    for snapshot in &snapshots {
        name_lookup.insert(snapshot.entity, snapshot.name.clone());
    }

    for trade in planned_trades {
        let price = trade.price as i64;

        if let Ok((
            mut seller_alloc,
            mut seller_reservations,
            mut seller_stockpile,
            mut seller_workforce,
            mut seller_treasury,
            _,
        )) = nations.get_mut(trade.seller)
        {
            if let Some(vec) = seller_alloc.market_sells.get_mut(&trade.good) {
                if let Some(pos) = vec.iter().position(|id| *id == trade.reservation) {
                    vec.swap_remove(pos);
                } else {
                    warn!(
                        "Market trade missing reservation for seller {:?} {:?}",
                        trade.seller, trade.good
                    );
                }
            }

            seller_reservations.consume(
                trade.reservation,
                &mut seller_stockpile,
                &mut seller_workforce,
                &mut seller_treasury,
            );
            seller_treasury.add(price);
        } else {
            warn!("Market trade failed: seller {:?} not found", trade.seller);
            continue;
        }

        if let Ok((_, _, mut buyer_stockpile, _, mut buyer_treasury, _)) =
            nations.get_mut(trade.buyer)
        {
            buyer_stockpile.add(trade.good, 1);
            buyer_treasury.subtract(price);
        } else {
            warn!("Market trade failed: buyer {:?} not found", trade.buyer);
            continue;
        }

        let seller_name = name_lookup
            .get(&trade.seller)
            .and_then(|name| name.as_ref())
            .map_or_else(
                || format!("#{:?}", trade.seller.index()),
                ToString::to_string,
            );
        let buyer_name = name_lookup
            .get(&trade.buyer)
            .and_then(|name| name.as_ref())
            .map_or_else(
                || format!("#{:?}", trade.buyer.index()),
                ToString::to_string,
            );

        info!(
            "Market trade: {seller_name} sold 1 {:?} to {buyer_name} for ${}",
            trade.good, trade.price
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{App, Entity, Query, Res, With};

    use crate::economy::market::{MarketPriceModel, MarketVolume};
    use crate::economy::trade::resolve_market_orders;
    use crate::economy::{
        Good,
        allocation::Allocations,
        nation::{Name, NationId},
        reservation::ReservationSystem,
        stockpile::Stockpile,
        treasury::Treasury,
        workforce::Workforce,
    };

    #[test]
    fn sells_goods_and_transfers_cash() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());

        let seller = app
            .world_mut()
            .spawn((
                NationId(1),
                Name("Seller".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        let buyer = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(seller)
                .unwrap()
                .add(Good::Grain, 5);
        }

        {
            let world = app.world_mut();
            let mut seller_query = world.query::<(
                &mut Stockpile,
                &mut ReservationSystem,
                &mut Allocations,
                &mut Workforce,
                &mut Treasury,
            )>();

            let (mut stockpile, mut reservations, mut allocations, mut workforce, mut treasury) =
                seller_query.get_mut(world, seller).expect("seller data");

            if let Some(res_id) = reservations.try_reserve(
                vec![(Good::Grain, 1u32)],
                0,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            ) {
                allocations
                    .market_sells
                    .entry(Good::Grain)
                    .or_default()
                    .push(res_id);
            } else {
                panic!("Failed to reserve grain for sale");
            }

            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Grain, 1);
        }

        let mut system_state: SystemState<(
            Query<
                (
                    &mut Allocations,
                    &mut ReservationSystem,
                    &mut Stockpile,
                    &mut Workforce,
                    &mut Treasury,
                    Option<&Name>,
                ),
                With<NationId>,
            >,
            Query<Entity, With<NationId>>,
            Res<MarketPriceModel>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing) = system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing);
            system_state.apply(app.world_mut());
        }

        let world = app.world();
        let seller_stockpile = world.get::<Stockpile>(seller).unwrap();
        let buyer_stockpile = world.get::<Stockpile>(buyer).unwrap();
        let seller_treasury = world.get::<Treasury>(seller).unwrap();
        let buyer_treasury = world.get::<Treasury>(buyer).unwrap();

        assert_eq!(seller_stockpile.get(Good::Grain), 4);
        assert_eq!(buyer_stockpile.get(Good::Grain), 1);

        let price =
            MarketPriceModel::default().price_for(Good::Grain, MarketVolume::default()) as i64;
        assert_eq!(seller_treasury.total(), 1_000 + price);
        assert_eq!(buyer_treasury.total(), 1_000 - price);
    }

    #[test]
    fn buys_multiple_units_when_requested() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());

        let seller = app
            .world_mut()
            .spawn((
                NationId(1),
                Name("Seller".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        let buyer = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(seller)
                .unwrap()
                .add(Good::Grain, 5);
        }

        {
            let world = app.world_mut();
            let mut seller_query = world.query::<(
                &mut Stockpile,
                &mut ReservationSystem,
                &mut Allocations,
                &mut Workforce,
                &mut Treasury,
            )>();

            let (mut stockpile, mut reservations, mut allocations, mut workforce, mut treasury) =
                seller_query.get_mut(world, seller).expect("seller data");

            for _ in 0..2 {
                let res_id = reservations
                    .try_reserve(
                        vec![(Good::Grain, 1u32)],
                        0,
                        0,
                        &mut stockpile,
                        &mut workforce,
                        &mut treasury,
                    )
                    .expect("reserve grain for sale");
                allocations
                    .market_sells
                    .entry(Good::Grain)
                    .or_default()
                    .push(res_id);
            }

            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Grain, 2);
        }

        let mut system_state: SystemState<(
            Query<
                (
                    &mut Allocations,
                    &mut ReservationSystem,
                    &mut Stockpile,
                    &mut Workforce,
                    &mut Treasury,
                    Option<&Name>,
                ),
                With<NationId>,
            >,
            Query<Entity, With<NationId>>,
            Res<MarketPriceModel>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing) = system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing);
            system_state.apply(app.world_mut());
        }

        let world = app.world();
        let seller_stockpile = world.get::<Stockpile>(seller).unwrap();
        let buyer_stockpile = world.get::<Stockpile>(buyer).unwrap();
        let seller_treasury = world.get::<Treasury>(seller).unwrap();
        let buyer_treasury = world.get::<Treasury>(buyer).unwrap();

        assert_eq!(seller_stockpile.get(Good::Grain), 3);
        assert_eq!(buyer_stockpile.get(Good::Grain), 2);

        let price =
            MarketPriceModel::default().price_for(Good::Grain, MarketVolume::default()) as i64;
        assert_eq!(seller_treasury.total(), 1_000 + price * 2);
        assert_eq!(buyer_treasury.total(), 1_000 - price * 2);
    }
}
