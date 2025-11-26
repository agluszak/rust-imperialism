use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::nation::{Name, NationId};
use crate::economy::trade_capacity::TradeCapacity;
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
///
/// After resolution, base prices are updated based on observed supply/demand.
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
    mut pricing: ResMut<MarketPriceModel>,
    mut trade_capacity: ResMut<TradeCapacity>,
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

    trade_capacity.reset_usage();

    let mut capacity_available: HashMap<Entity, u32> = HashMap::new();
    for entity in nation_entities.iter() {
        let available = trade_capacity.available(entity);
        capacity_available.insert(entity, available);
    }

    let mut cash_map: HashMap<Entity, i64> = snapshots
        .iter()
        .map(|snapshot| (snapshot.entity, snapshot.available_cash))
        .collect();
    let mut planned_trades: Vec<PlannedTrade> = Vec::new();

    // Track supply/demand volumes for price adjustment at end
    let mut observed_volumes: HashMap<Good, MarketVolume> = HashMap::new();

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
            let supply_units: u32 = sellers.iter().map(|(_, r)| r.len() as u32).sum();
            info!(
                "Market {:?}: {} sellers but no buyers (supply: {} units)",
                good,
                sellers.len(),
                supply_units
            );
            // Record supply with no demand - prices should drop
            observed_volumes.insert(good, MarketVolume::new(supply_units, 0));
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
        observed_volumes.insert(good, volume);

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

                let seller_capacity = capacity_available.get(&seller).copied().unwrap_or(0);
                let buyer_capacity = capacity_available.get(&buyer).copied().unwrap_or(0);

                debug!(
                    "Market capacity {:?}: seller {:?} cap {}, buyer {:?} cap {}",
                    good, seller, seller_capacity, buyer, buyer_capacity
                );

                if seller_capacity == 0 {
                    debug!(
                        "Skipping seller {:?} for {:?}: no trade capacity available",
                        seller, good
                    );
                    continue;
                }

                if buyer_capacity == 0 {
                    cash_map.insert(buyer, cash_available);
                    continue 'buyers;
                }

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

                if let Some(entry) = capacity_available.get_mut(&seller) {
                    *entry = entry.saturating_sub(1);
                }
                if let Some(entry) = capacity_available.get_mut(&buyer) {
                    *entry = entry.saturating_sub(1);
                }
                let seller_consumed = trade_capacity.consume(seller, 1);
                let buyer_consumed = trade_capacity.consume(buyer, 1);
                debug_assert!(seller_consumed && buyer_consumed, "trade capacity mismatch");

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
        // Still update prices based on observed supply/demand even if no trades completed
        for (good, volume) in observed_volumes {
            let old_price = pricing.current_price(good);
            pricing.update_price_from_volume(good, volume);
            let new_price = pricing.current_price(good);
            if old_price != new_price {
                info!(
                    "Market {:?}: price adjusted ${} → ${} (supply: {}, demand: {}, no trades)",
                    good, old_price, new_price, volume.supply_units, volume.demand_units
                );
            }
        }
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

    // Update base prices for next turn based on observed supply/demand
    for (good, volume) in observed_volumes {
        let old_price = pricing.current_price(good);
        pricing.update_price_from_volume(good, volume);
        let new_price = pricing.current_price(good);
        if old_price != new_price {
            info!(
                "Market {:?}: price adjusted ${} → ${} (supply: {}, demand: {})",
                good, old_price, new_price, volume.supply_units, volume.demand_units
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{App, Entity, Query, ResMut, With};

    use crate::economy::market::MarketPriceModel;
    use crate::economy::trade::resolve_market_orders;
    use crate::economy::trade_capacity::TradeCapacity;

    fn set_trade_capacity(app: &mut App, nation: Entity, total: u32) {
        let world = app.world_mut();
        let mut capacity = world.resource_mut::<TradeCapacity>();
        let snapshot = capacity.snapshot_mut(nation);
        snapshot.total = total;
        snapshot.used = 0;
    }
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
        app.insert_resource(TradeCapacity::default());

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

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer, 5);

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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
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
        app.insert_resource(TradeCapacity::default());

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

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer, 5);

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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
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
    fn trade_respects_trade_capacity_limits() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

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

        set_trade_capacity(&mut app, seller, 1);
        set_trade_capacity(&mut app, buyer, 1);

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

            stockpile.add(Good::Grain, 4);
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
        }

        {
            app.world_mut()
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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
            system_state.apply(app.world_mut());
        }

        let world = app.world();
        let buyer_stockpile = world.get::<Stockpile>(buyer).unwrap();
        let trade_capacity = world.resource::<TradeCapacity>();
        let seller_snapshot = trade_capacity.snapshot(seller);
        let buyer_snapshot = trade_capacity.snapshot(buyer);

        assert_eq!(seller_snapshot.total, 1);
        assert_eq!(buyer_snapshot.total, 1);
        assert_eq!(seller_snapshot.used, 1);
        assert_eq!(buyer_snapshot.used, 1);
        assert_eq!(
            buyer_stockpile.get(Good::Grain),
            1,
            "Only one unit should arrive"
        );

        let seller_allocations = world.get::<Allocations>(seller).unwrap();
        assert_eq!(seller_allocations.market_sell_count(Good::Grain), 1);
    }

    #[test]
    fn market_matches_seller_with_late_buyer() {
        // This test verifies the fix for the turn phase timing issue:
        // Seller expresses interest first, buyer expresses interest later,
        // market should still match them correctly
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

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

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer, 5);

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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
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
        assert!(
            seller_gain > 0,
            "Seller should have earned money from the sale"
        );
    }

    #[test]
    fn processes_goods_in_manual_order() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

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
                Treasury::new(80),
            ))
            .id();

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer, 5);

        // Seller reserves one Grain and one Cotton for sale.
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

            stockpile.add(Good::Grain, 1);
            stockpile.add(Good::Cotton, 1);

            for good in [Good::Grain, Good::Cotton] {
                let res_id = reservations
                    .try_reserve(
                        vec![(good, 1u32)],
                        0,
                        0,
                        &mut stockpile,
                        &mut workforce,
                        &mut treasury,
                    )
                    .expect("reserve good for sale");
                allocations
                    .market_sells
                    .entry(good)
                    .or_default()
                    .push(res_id);
            }
        }

        // Buyer wants both commodities but only has enough cash for one unit.
        {
            let world = app.world_mut();
            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Grain);
            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Cotton);
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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
            system_state.apply(app.world_mut());
        }

        let world = app.world();
        let buyer_stockpile = world.get::<Stockpile>(buyer).unwrap();
        let seller_treasury = world.get::<Treasury>(seller).unwrap();

        assert_eq!(buyer_stockpile.get(Good::Grain), 1);
        assert_eq!(buyer_stockpile.get(Good::Cotton), 0);
        assert_eq!(seller_treasury.total(), 1_000 + 60);
    }

    #[test]
    fn multiple_buyers_raise_price() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

        let seller = app
            .world_mut()
            .spawn((
                NationId(1),
                Name("Seller".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(0),
            ))
            .id();

        let buyer_a = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer A".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        let buyer_b = app
            .world_mut()
            .spawn((
                NationId(3),
                Name("Buyer B".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(1_000),
            ))
            .id();

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer_a, 5);
        set_trade_capacity(&mut app, buyer_b, 5);

        // Seller reserves one Coal for sale at the start of the turn.
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

            stockpile.add(Good::Coal, 1);
            let res_id = reservations
                .try_reserve(
                    vec![(Good::Coal, 1u32)],
                    0,
                    0,
                    &mut stockpile,
                    &mut workforce,
                    &mut treasury,
                )
                .expect("reserve coal for sale");
            allocations
                .market_sells
                .entry(Good::Coal)
                .or_default()
                .push(res_id);
        }

        // Both buyers express interest in Coal, pushing demand above supply.
        {
            let world = app.world_mut();
            world
                .get_mut::<Allocations>(buyer_a)
                .unwrap()
                .market_buys
                .insert(Good::Coal);
            world
                .get_mut::<Allocations>(buyer_b)
                .unwrap()
                .market_buys
                .insert(Good::Coal);
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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
            system_state.apply(app.world_mut());
        }

        let world = app.world();
        let seller_treasury = world.get::<Treasury>(seller).unwrap();
        let buyer_a_stockpile = world.get::<Stockpile>(buyer_a).unwrap();
        let buyer_b_stockpile = world.get::<Stockpile>(buyer_b).unwrap();

        assert_eq!(seller_treasury.total(), 113);
        assert_eq!(
            buyer_a_stockpile.get(Good::Coal) + buyer_b_stockpile.get(Good::Coal),
            1
        );
    }

    #[test]
    fn prices_adjust_based_on_supply_demand() {
        // Test that prices rise when demand exceeds supply
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

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

        let buyer_a = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer A".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(5_000),
            ))
            .id();

        let buyer_b = app
            .world_mut()
            .spawn((
                NationId(3),
                Name("Buyer B".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(5_000),
            ))
            .id();

        set_trade_capacity(&mut app, seller, 5);
        set_trade_capacity(&mut app, buyer_a, 5);
        set_trade_capacity(&mut app, buyer_b, 5);

        // Record initial price
        let initial_price = app
            .world()
            .resource::<MarketPriceModel>()
            .current_price(Good::Iron);

        // Seller reserves just 1 Iron, but two buyers want it (high demand, low supply)
        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(seller)
                .unwrap()
                .add(Good::Iron, 1);

            let mut seller_query = world.query::<(
                &mut Stockpile,
                &mut ReservationSystem,
                &mut Allocations,
                &mut Workforce,
                &mut Treasury,
            )>();

            let (mut stockpile, mut reservations, mut allocations, mut workforce, mut treasury) =
                seller_query.get_mut(world, seller).expect("seller data");

            let res_id = reservations
                .try_reserve(
                    vec![(Good::Iron, 1u32)],
                    0,
                    0,
                    &mut stockpile,
                    &mut workforce,
                    &mut treasury,
                )
                .expect("reserve iron for sale");
            allocations
                .market_sells
                .entry(Good::Iron)
                .or_default()
                .push(res_id);

            // Both buyers express interest
            world
                .get_mut::<Allocations>(buyer_a)
                .unwrap()
                .market_buys
                .insert(Good::Iron);
            world
                .get_mut::<Allocations>(buyer_b)
                .unwrap()
                .market_buys
                .insert(Good::Iron);
        }

        // Run market resolution
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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
            system_state.apply(app.world_mut());
        }

        // Price should have increased due to high demand (2 buyers), low supply (1 unit)
        let new_price = app
            .world()
            .resource::<MarketPriceModel>()
            .current_price(Good::Iron);
        assert!(
            new_price > initial_price,
            "Price should increase when demand exceeds supply: {} should be > {}",
            new_price,
            initial_price
        );
    }

    #[test]
    fn prices_drop_when_supply_exceeds_demand() {
        let mut app = App::new();
        app.insert_resource(MarketPriceModel::default());
        app.insert_resource(TradeCapacity::default());

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

        // Only one buyer with limited cash
        let buyer = app
            .world_mut()
            .spawn((
                NationId(2),
                Name("Buyer".into()),
                Allocations::default(),
                ReservationSystem::default(),
                Stockpile::default(),
                Workforce::new(),
                Treasury::new(100), // Can only afford 1 unit at ~60 price
            ))
            .id();

        set_trade_capacity(&mut app, seller, 10);
        set_trade_capacity(&mut app, buyer, 10);

        // Record initial price
        let initial_price = app
            .world()
            .resource::<MarketPriceModel>()
            .current_price(Good::Grain);

        // Seller has lots of grain for sale (high supply)
        {
            let world = app.world_mut();
            world
                .get_mut::<Stockpile>(seller)
                .unwrap()
                .add(Good::Grain, 10);

            let mut seller_query = world.query::<(
                &mut Stockpile,
                &mut ReservationSystem,
                &mut Allocations,
                &mut Workforce,
                &mut Treasury,
            )>();

            let (mut stockpile, mut reservations, mut allocations, mut workforce, mut treasury) =
                seller_query.get_mut(world, seller).expect("seller data");

            // Sell 5 units - high supply
            for _ in 0..5 {
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

            // Just one buyer with limited buying power
            world
                .get_mut::<Allocations>(buyer)
                .unwrap()
                .market_buys
                .insert(Good::Grain);
        }

        // Run market resolution
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
            ResMut<MarketPriceModel>,
            ResMut<TradeCapacity>,
        )> = SystemState::new(app.world_mut());

        {
            let (nations, nation_entities, pricing, trade_capacity) =
                system_state.get_mut(app.world_mut());
            resolve_market_orders(nations, nation_entities, pricing, trade_capacity);
            system_state.apply(app.world_mut());
        }

        // Price should have dropped due to high supply (5 units), low demand
        let new_price = app
            .world()
            .resource::<MarketPriceModel>()
            .current_price(Good::Grain);
        assert!(
            new_price < initial_price,
            "Price should decrease when supply exceeds demand: {} should be < {}",
            new_price,
            initial_price
        );
    }
}
