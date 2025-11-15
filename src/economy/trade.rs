use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

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
    buy_interest: HashSet<Good>,
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
            let buy_interest: HashSet<Good> = allocations.market_buys.clone();

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
                buy_interest,
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

        // Collect buyers who have expressed interest
        let interested_buyers: Vec<Entity> = snapshots
            .iter()
            .filter_map(|snapshot| {
                if snapshot.buy_interest.contains(&good) {
                    Some(snapshot.entity)
                } else {
                    None
                }
            })
            .collect();

        if interested_buyers.is_empty() {
            info!(
                "Market {:?}: {} sellers but no buyers (supply: {} units)",
                good,
                sellers.len(),
                sellers.iter().map(|(_, r)| r.len()).sum::<usize>()
            );
            continue;
        }

        let total_supply: u32 = sellers
            .iter()
            .map(|(_, reservations)| reservations.len() as u32)
            .sum();

        // For pricing, estimate total demand based on what buyers could afford at base price
        let base_price = pricing.price_for(good, MarketVolume::default()) as i64;
        let total_demand: u32 = interested_buyers
            .iter()
            .filter_map(|&buyer| cash_map.get(&buyer))
            .map(|&cash| {
                if base_price > 0 {
                    (cash / base_price).max(1) as u32
                } else {
                    1
                }
            })
            .sum();

        let volume = MarketVolume::new(total_supply, total_demand);
        let price = pricing.price_for(good, volume) as i64;
        if price <= 0 {
            continue;
        }

        sellers.sort_by_key(|(entity, _)| entity.index());

        let mut seller_queue: VecDeque<(Entity, Vec<ReservationId>)> =
            sellers.into_iter().collect();

        // Each interested buyer tries to buy as much as they can afford
        'buyers: for buyer in interested_buyers {
            let Some(mut cash_available) = cash_map.get(&buyer).copied() else {
                continue;
            };

            loop {
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
                    continue;
                };

                planned_trades.push(PlannedTrade {
                    good,
                    price: price as u32,
                    seller,
                    buyer,
                    reservation,
                });

                info!(
                    "Market trade: {:?} sold for ${} (seller: {:?}, buyer: {:?})",
                    good, price, seller, buyer
                );

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

    use crate::economy::market::MarketPriceModel;
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
                .insert(Good::Grain);
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

        // Verify that 1 unit was traded
        assert_eq!(seller_stockpile.get(Good::Grain), 4);
        assert_eq!(buyer_stockpile.get(Good::Grain), 1);

        // With boolean buy interest, pricing is based on estimated demand
        // Just verify that money was transferred correctly
        let seller_gain = seller_treasury.total() - 1_000;
        let buyer_cost = 1_000 - buyer_treasury.total();
        assert_eq!(seller_gain, buyer_cost, "Money transfer mismatch");
        assert!(seller_gain > 0, "Seller should have earned money");
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
                .insert(Good::Grain);
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

        // Verify that 2 units were traded
        assert_eq!(seller_stockpile.get(Good::Grain), 3);
        assert_eq!(buyer_stockpile.get(Good::Grain), 2);

        // With boolean buy interest, pricing is based on estimated demand
        // Just verify that money was transferred correctly
        let seller_gain = seller_treasury.total() - 1_000;
        let buyer_cost = 1_000 - buyer_treasury.total();
        assert_eq!(seller_gain, buyer_cost, "Money transfer mismatch");
        assert!(seller_gain > 0, "Seller should have earned money");
    }

    #[test]
    fn market_matches_seller_with_late_buyer() {
        // This test verifies the fix for the turn phase timing issue:
        // Seller expresses interest first, buyer expresses interest later,
        // market should still match them correctly
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());

        let seller = app
            .world_mut()
            .spawn((
                NationId(1),
                Name("Seller Nation".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(500),
            ))
            .id();

        let buyer = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer Nation".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        // Setup: Seller adds stock and reserves it for sale (simulating PlayerTurn)
        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(seller)
                .unwrap()
                .add(Good::Coal, 10);

            let mut seller_query = world.query::<(
                &mut Stockpile,
                &mut ReservationSystem,
                &mut Allocations,
                &mut Workforce,
                &mut Treasury,
            )>();

            let (mut stockpile, mut reservations, mut allocations, mut workforce, mut treasury) =
                seller_query.get_mut(world, seller).expect("seller data");

            // Seller reserves 3 Coal for sale
            for _ in 0..3 {
                if let Some(res_id) = reservations.try_reserve(
                    vec![(Good::Coal, 1u32)],
                    0,
                    0,
                    &mut stockpile,
                    &mut workforce,
                    &mut treasury,
                ) {
                    allocations
                        .market_sells
                        .entry(Good::Coal)
                        .or_default()
                        .push(res_id);
                }
            }
        }

        // Buyer expresses interest (simulating EnemyTurn - happens AFTER seller's sell orders)
        {
            let world = app.world_mut();
            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Coal);
        }

        // Market resolution (should happen at start of next PlayerTurn, AFTER both decided)
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

        // Verify: Trade should have executed successfully
        let world = app.world();
        let seller_stockpile = world.get::<Stockpile>(seller).unwrap();
        let buyer_stockpile = world.get::<Stockpile>(buyer).unwrap();
        let seller_treasury = world.get::<Treasury>(seller).unwrap();
        let buyer_treasury = world.get::<Treasury>(buyer).unwrap();

        // Buyer should have purchased all 3 units (or as many as they could afford)
        let units_bought = buyer_stockpile.get(Good::Coal);
        assert!(
            units_bought > 0,
            "Buyer should have successfully purchased Coal despite expressing interest late"
        );
        assert_eq!(
            seller_stockpile.get(Good::Coal),
            10 - units_bought,
            "Seller should have lost the units that were sold"
        );

        // Money should have been transferred
        let seller_gain = seller_treasury.total() - 500;
        let buyer_cost = 1_000 - buyer_treasury.total();
        assert_eq!(
            seller_gain, buyer_cost,
            "Money transferred should match: seller gain = buyer cost"
        );
        assert!(seller_gain > 0, "Seller should have earned money from the sale");
    }
}
