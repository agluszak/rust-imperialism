use bevy::ecs::system::SystemState;
use bevy::prelude::{Query, ResMut, World};

use crate::economy::{
    allocation::Allocations,
    allocation_systems::{calculate_inputs_for_one_unit, execute_queued_production_orders},
    goods::Good,
    nation::{Nation, NationInstance},
    production::{Building, BuildingKind, Buildings},
    reservation::ReservationSystem,
    stockpile::Stockpile,
    treasury::Treasury,
    workforce::Workforce,
};
use crate::messages::AdjustProduction;
use crate::orders::OrdersQueue;

/// Test the intelligent input selection logic for Textile Mill
#[test]
fn test_textile_mill_prefers_cotton() {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Cotton, 100);
    stockpile.add(Good::Wool, 50);

    let inputs = calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0], (Good::Cotton, 2));
}

#[test]
fn test_textile_mill_falls_back_to_wool() {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Cotton, 0); // No cotton
    stockpile.add(Good::Wool, 100);

    let inputs = calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0], (Good::Wool, 2));
}

#[test]
fn test_textile_mill_uses_wool_when_more_available() {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Cotton, 1); // Less than needed
    stockpile.add(Good::Wool, 100);

    let inputs = calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

    // Should use Wool because Cotton < 2
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0], (Good::Wool, 2));
}

#[test]
fn test_food_processing_prefers_fish() {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Grain, 100);
    stockpile.add(Good::Fruit, 100);
    stockpile.add(Good::Fish, 50);
    stockpile.add(Good::Livestock, 50);

    let inputs = calculate_inputs_for_one_unit(
        BuildingKind::FoodProcessingCenter,
        Good::CannedFood,
        &stockpile,
    );

    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0], (Good::Grain, 2));
    assert_eq!(inputs[1], (Good::Fruit, 1));
    assert_eq!(inputs[2], (Good::Fish, 1)); // Should prefer Fish
}

#[test]
fn test_food_processing_falls_back_to_livestock() {
    let mut stockpile = Stockpile::default();
    stockpile.add(Good::Grain, 100);
    stockpile.add(Good::Fruit, 100);
    stockpile.add(Good::Fish, 0); // No fish
    stockpile.add(Good::Livestock, 50);

    let inputs = calculate_inputs_for_one_unit(
        BuildingKind::FoodProcessingCenter,
        Good::CannedFood,
        &stockpile,
    );

    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[2], (Good::Livestock, 1)); // Should use Livestock
}

#[test]
fn test_lumber_mill_lumber_output() {
    let stockpile = Stockpile::default();

    let inputs = calculate_inputs_for_one_unit(BuildingKind::LumberMill, Good::Lumber, &stockpile);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0], (Good::Timber, 2));
}

#[test]
fn test_lumber_mill_paper_output() {
    let stockpile = Stockpile::default();

    let inputs = calculate_inputs_for_one_unit(BuildingKind::LumberMill, Good::Paper, &stockpile);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0], (Good::Timber, 2));
}

#[test]
fn test_steel_mill_inputs() {
    let stockpile = Stockpile::default();

    let inputs = calculate_inputs_for_one_unit(BuildingKind::SteelMill, Good::Steel, &stockpile);

    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0], (Good::Iron, 1));
    assert_eq!(inputs[1], (Good::Coal, 1));
}

/// Test reservation system integration
#[test]
fn test_reservation_system_basics() {
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    stockpile.add(Good::Cotton, 10);
    workforce.add_untrained(5);
    workforce.update_labor_pool(); // Update pool after adding workers

    // Reserve 2 Cotton + 1 Labor
    let res_id = reservations.try_reserve(
        vec![(Good::Cotton, 2)],
        1,
        0,
        &mut stockpile,
        &mut workforce,
        &mut treasury,
    );

    assert!(res_id.is_some(), "Should be able to reserve resources");
    assert_eq!(stockpile.get_available(Good::Cotton), 8);
    assert_eq!(workforce.labor_pool.available(), 4); // Check labor pool, not recalculated value
}

#[test]
fn test_reservation_fails_on_insufficient_resources() {
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    stockpile.add(Good::Cotton, 1); // Not enough
    workforce.add_untrained(5);
    workforce.update_labor_pool(); // Update pool after adding workers

    // Try to reserve 2 Cotton (but only 1 available)
    let res_id = reservations.try_reserve(
        vec![(Good::Cotton, 2)],
        1,
        0,
        &mut stockpile,
        &mut workforce,
        &mut treasury,
    );

    assert!(res_id.is_none(), "Should fail to reserve");
    // Verify nothing was reserved (rollback)
    assert_eq!(stockpile.get_available(Good::Cotton), 1);
    assert_eq!(workforce.labor_pool.available(), 5); // Check labor pool, not recalculated value
}

#[test]
fn test_reservation_release() {
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    stockpile.add(Good::Cotton, 10);
    workforce.add_untrained(5);
    workforce.update_labor_pool(); // Update pool after adding workers

    // Reserve resources
    let res_id = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();

    assert_eq!(stockpile.get_available(Good::Cotton), 8);
    assert_eq!(workforce.labor_pool.available(), 4); // Check labor pool, not recalculated value

    // Release reservation
    reservations.release(res_id, &mut stockpile, &mut workforce, &mut treasury);

    assert_eq!(stockpile.get_available(Good::Cotton), 10);
    assert_eq!(workforce.labor_pool.available(), 5); // Check labor pool, not recalculated value
}

#[test]
fn test_allocations_tracking() {
    use bevy::prelude::Entity;
    let mut allocations = Allocations::default();
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    // Mock entities
    let building = Entity::from_bits(1);

    stockpile.add(Good::Cotton, 20);
    workforce.add_untrained(5);
    workforce.update_labor_pool(); // Update pool after adding workers

    // Add some production allocations using real reservations
    let res1 = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();
    let res2 = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();

    allocations
        .production
        .entry((building, Good::Fabric))
        .or_default()
        .push(res1);
    allocations
        .production
        .entry((building, Good::Fabric))
        .or_default()
        .push(res2);

    // Check count
    assert_eq!(allocations.production_count(building, Good::Fabric), 2);
    assert_eq!(allocations.production_count(building, Good::Lumber), 0);
}

#[test]
fn test_buildings_collection() {
    let mut buildings = Buildings::new();

    // Add buildings
    buildings.insert(Building::textile_mill(10));
    buildings.insert(Building::lumber_mill(8));

    // Check retrieval
    assert!(buildings.get(BuildingKind::TextileMill).is_some());
    assert!(buildings.get(BuildingKind::LumberMill).is_some());
    assert!(buildings.get(BuildingKind::SteelMill).is_none());

    // Check capacity
    assert_eq!(
        buildings.get(BuildingKind::TextileMill).unwrap().capacity,
        10
    );
}

#[test]
fn test_buildings_with_all_initial() {
    let buildings = Buildings::with_all_initial();

    // Should have all production buildings
    assert!(buildings.get(BuildingKind::TextileMill).is_some());
    assert!(buildings.get(BuildingKind::LumberMill).is_some());
    assert!(buildings.get(BuildingKind::SteelMill).is_some());
    assert!(buildings.get(BuildingKind::FoodProcessingCenter).is_some());
    assert!(buildings.get(BuildingKind::ClothingFactory).is_some());
    assert!(buildings.get(BuildingKind::FurnitureFactory).is_some());
    assert!(buildings.get(BuildingKind::MetalWorks).is_some());
    assert!(buildings.get(BuildingKind::Refinery).is_some());
    assert!(buildings.get(BuildingKind::Railyard).is_some());
    assert!(buildings.get(BuildingKind::Shipyard).is_some());

    // Check default capacities
    assert_eq!(
        buildings.get(BuildingKind::TextileMill).unwrap().capacity,
        8
    );
    assert_eq!(buildings.get(BuildingKind::LumberMill).unwrap().capacity, 4);
    assert_eq!(buildings.get(BuildingKind::SteelMill).unwrap().capacity, 4);
    assert_eq!(
        buildings
            .get(BuildingKind::FoodProcessingCenter)
            .unwrap()
            .capacity,
        4
    );
    assert_eq!(
        buildings
            .get(BuildingKind::ClothingFactory)
            .unwrap()
            .capacity,
        2
    );
    assert_eq!(
        buildings
            .get(BuildingKind::FurnitureFactory)
            .unwrap()
            .capacity,
        2
    );
    assert_eq!(buildings.get(BuildingKind::MetalWorks).unwrap().capacity, 2);
    assert_eq!(buildings.get(BuildingKind::Refinery).unwrap().capacity, 2);
    assert_eq!(
        buildings.get(BuildingKind::Railyard).unwrap().capacity,
        u32::MAX
    );
    assert_eq!(
        buildings.get(BuildingKind::Shipyard).unwrap().capacity,
        u32::MAX
    );
}

#[test]
fn test_multiple_reservations_for_same_building() {
    use bevy::prelude::Entity;
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    stockpile.add(Good::Cotton, 20);
    workforce.add_untrained(10);
    workforce.update_labor_pool(); // Update pool after adding workers

    // Make multiple reservations (simulating multiple units)
    let res1 = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();
    let res2 = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();
    let res3 = reservations
        .try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        )
        .unwrap();

    // Should have reserved 6 Cotton and 3 Labor
    assert_eq!(stockpile.get_available(Good::Cotton), 14);
    assert_eq!(workforce.labor_pool.available(), 7); // Check labor pool, not recalculated value

    // Track in allocations
    let mut allocations = Allocations::default();
    let building = Entity::from_bits(1);
    allocations
        .production
        .entry((building, Good::Fabric))
        .or_default()
        .extend([res1, res2, res3]);

    assert_eq!(allocations.production_count(building, Good::Fabric), 3);
}

/// Test that multi-output buildings (Lumber Mill) cannot exceed total capacity
/// This test verifies the core capacity checking logic used in the adjustment system
#[test]
fn test_lumber_mill_capacity_across_multiple_outputs() {
    use bevy::prelude::Entity;

    let mut allocations = Allocations::default();
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(1000);

    // Setup: Plenty of resources
    stockpile.add(Good::Timber, 100);
    workforce.add_untrained(20);
    workforce.update_labor_pool();

    let building_entity = Entity::from_bits(42);
    let lumber_mill_capacity = 4u32;

    // Simulate allocating 4 Lumber first
    for _ in 0..4 {
        let res = reservations
            .try_reserve(
                vec![(Good::Timber, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        allocations
            .production
            .entry((building_entity, Good::Lumber))
            .or_default()
            .push(res);
    }

    // Now simulate the capacity check that should happen when trying to add Paper
    // This is the logic from apply_production_adjustments after the fix

    // Calculate total current production for this building across ALL outputs
    let mut total_building_production = 0u32;
    for ((entity, _output), res_ids) in allocations.production.iter() {
        if *entity == building_entity {
            total_building_production += res_ids.len() as u32;
        }
    }

    // Paper has 0 current allocations
    let paper_current_count = allocations.production_count(building_entity, Good::Paper);
    let paper_current_count_u32 = paper_current_count as u32;

    // Calculate remaining capacity (excluding current allocation for Paper, which is 0)
    let other_outputs = total_building_production - paper_current_count_u32;
    let remaining_capacity = lumber_mill_capacity.saturating_sub(other_outputs);

    // Try to allocate 1 Paper - should be capped by remaining capacity
    let target_paper = 1u32.min(remaining_capacity);

    println!(
        "Lumber Mill (capacity {}): Lumber={}, total_production={}, remaining_capacity={}, target_paper={}",
        lumber_mill_capacity,
        allocations.production_count(building_entity, Good::Lumber),
        total_building_production,
        remaining_capacity,
        target_paper
    );

    // The capacity check should prevent adding Paper when Lumber uses full capacity
    assert_eq!(
        remaining_capacity, 0,
        "Remaining capacity should be 0 when Lumber uses all capacity"
    );
    assert_eq!(
        target_paper, 0,
        "Target for Paper should be capped to 0 when no capacity remains"
    );

    // Verify total doesn't exceed capacity
    let lumber_count = allocations.production_count(building_entity, Good::Lumber);
    let paper_count = allocations.production_count(building_entity, Good::Paper);
    let total_count = lumber_count + paper_count;

    assert!(
        total_count <= lumber_mill_capacity as usize,
        "Total production ({}) exceeds building capacity ({})",
        total_count,
        lumber_mill_capacity
    );
}

/// Test that buy and sell orders are mutually exclusive for the same good
#[test]
fn test_market_orders_mutually_exclusive() {
    let mut allocations = Allocations::default();
    let mut reservations = ReservationSystem::default();
    let mut stockpile = Stockpile::default();
    let mut workforce = Workforce::new();
    let mut treasury = Treasury::new(10000);

    // Setup: plenty of resources
    stockpile.add(Good::Cotton, 100);
    workforce.add_untrained(10);
    workforce.update_labor_pool();

    // Place 3 sell orders for Cotton (uses reservation system)
    for _ in 0..3 {
        let res_id = reservations
            .try_reserve(
                vec![(Good::Cotton, 1)],
                0,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        allocations
            .market_sells
            .entry(Good::Cotton)
            .or_default()
            .push(res_id);
    }

    assert_eq!(allocations.market_sell_count(Good::Cotton), 3);
    assert!(!allocations.has_buy_interest(Good::Cotton));
    assert_eq!(stockpile.get_available(Good::Cotton), 97); // 3 reserved for selling

    // Set buy interest for Cotton - this should clear the sell orders
    allocations.market_buys.insert(Good::Cotton);

    // Simulate what apply_market_order_adjustments would do:
    // When setting buy interest, it should clear sell orders
    if let Some(sell_orders) = allocations.market_sells.get_mut(&Good::Cotton) {
        while let Some(res_id) = sell_orders.pop() {
            reservations.release(res_id, &mut stockpile, &mut workforce, &mut treasury);
        }
    }

    // Verify sell orders are now cleared and buy interest is set
    assert_eq!(allocations.market_sell_count(Good::Cotton), 0);
    assert!(allocations.has_buy_interest(Good::Cotton));
    assert_eq!(stockpile.get_available(Good::Cotton), 100); // All Cotton now available

    // Now reverse: place sell orders again, which should clear buy interest
    for _ in 0..2 {
        let res_id = reservations
            .try_reserve(
                vec![(Good::Cotton, 1)],
                0,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        allocations
            .market_sells
            .entry(Good::Cotton)
            .or_default()
            .push(res_id);
    }

    // Simulate what apply_market_order_adjustments would do:
    // When setting sell orders, it should clear buy interest
    allocations.market_buys.remove(&Good::Cotton);

    // Verify buy interest is now cleared and sell orders exist
    assert!(!allocations.has_buy_interest(Good::Cotton));
    assert_eq!(allocations.market_sell_count(Good::Cotton), 2);
    assert_eq!(stockpile.get_available(Good::Cotton), 98); // 2 reserved for selling
}

#[test]
fn execute_queued_production_orders_apply_and_clear() {
    let mut world = World::new();
    world.insert_resource(OrdersQueue::default());

    let building_entity = world.spawn(Building::textile_mill(8)).id();

    let nation_entity = world
        .spawn((
            Nation,
            Allocations::default(),
            ReservationSystem::default(),
            Stockpile::default(),
            Workforce::new(),
            Treasury::new(0),
        ))
        .id();

    {
        let mut stockpile = world
            .get_mut::<Stockpile>(nation_entity)
            .expect("stockpile not found");
        stockpile.add(Good::Cotton, 10);
        stockpile.add(Good::Wool, 10);
    }

    {
        let mut workforce = world
            .get_mut::<Workforce>(nation_entity)
            .expect("workforce not found");
        workforce.add_untrained(5);
        workforce.update_labor_pool();
    }

    let nation_instance = NationInstance::from_entity(world.entity(nation_entity))
        .expect("failed to build nation instance");

    world
        .resource_mut::<OrdersQueue>()
        .queue_production(AdjustProduction {
            nation: nation_instance,
            building: building_entity,
            output_good: Good::Fabric,
            target_output: 2,
        });

    let mut system_state = SystemState::<(
        ResMut<OrdersQueue>,
        Query<(
            &mut Allocations,
            &mut ReservationSystem,
            &mut Stockpile,
            &mut Workforce,
        )>,
        Query<&Building>,
    )>::new(&mut world);

    {
        let (orders, nations, buildings) = system_state.get_mut(&mut world);
        execute_queued_production_orders(orders, nations, buildings);
    }
    system_state.apply(&mut world);

    let allocations = world
        .get::<Allocations>(nation_entity)
        .expect("allocations not found");
    assert_eq!(
        allocations.production_count(building_entity, Good::Fabric),
        2
    );
    assert!(world.resource::<OrdersQueue>().is_empty());
}

#[test]
fn execute_queued_production_orders_respect_building_kind_capacity() {
    let mut world = World::new();
    world.insert_resource(OrdersQueue::default());

    let nation_entity = world
        .spawn((
            Nation,
            Allocations::default(),
            ReservationSystem::default(),
            Stockpile::default(),
            Workforce::new(),
            Treasury::new(0),
        ))
        .id();

    let food_factory = world.spawn(Building::food_processing_center(4)).id();
    let clothing_factory = world.spawn(Building::clothing_factory(2)).id();

    {
        let mut stockpile = world
            .get_mut::<Stockpile>(nation_entity)
            .expect("stockpile not found");
        stockpile.add(Good::Grain, 20);
        stockpile.add(Good::Fruit, 20);
        stockpile.add(Good::Fish, 20);
        stockpile.add(Good::Fabric, 20);
    }

    {
        let mut workforce = world
            .get_mut::<Workforce>(nation_entity)
            .expect("workforce not found");
        workforce.add_untrained(10);
        workforce.update_labor_pool();
    }

    let nation_instance = NationInstance::from_entity(world.entity(nation_entity))
        .expect("failed to build nation instance");

    world
        .resource_mut::<OrdersQueue>()
        .queue_production(AdjustProduction {
            nation: nation_instance,
            building: food_factory,
            output_good: Good::CannedFood,
            target_output: 4,
        });

    world
        .resource_mut::<OrdersQueue>()
        .queue_production(AdjustProduction {
            nation: nation_instance,
            building: clothing_factory,
            output_good: Good::Clothing,
            target_output: 2,
        });

    let mut system_state = SystemState::<(
        ResMut<OrdersQueue>,
        Query<(
            &mut Allocations,
            &mut ReservationSystem,
            &mut Stockpile,
            &mut Workforce,
        )>,
        Query<&Building>,
    )>::new(&mut world);

    {
        let (orders, nations, buildings) = system_state.get_mut(&mut world);
        execute_queued_production_orders(orders, nations, buildings);
    }
    system_state.apply(&mut world);

    let allocations = world
        .get::<Allocations>(nation_entity)
        .expect("allocations not found");
    assert_eq!(
        allocations.production_count(food_factory, Good::CannedFood),
        4
    );
    assert_eq!(
        allocations.production_count(clothing_factory, Good::Clothing),
        2
    );
}
