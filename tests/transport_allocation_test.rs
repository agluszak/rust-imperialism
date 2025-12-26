use bevy::prelude::*;
use rust_imperialism::{
    economy::{
        goods::Good,
        production::{ConnectedProduction, collect_connected_production},
        stockpile::Stockpile,
        transport::{TransportAllocations, TransportCapacity, TransportCommodity},
    },
    resources::ResourceType,
};

/// Test that resources are only collected when transport capacity is allocated
#[test]
fn test_resource_collection_requires_transport_allocation() {
    let mut app = App::new();

    // Initialize resources
    app.insert_resource(ConnectedProduction::default());
    app.insert_resource(TransportAllocations::default());
    app.insert_resource(TransportCapacity::default());

    // Create a nation with some connected production
    let nation = app.world_mut().spawn(Stockpile::default()).id();

    // Add connected production for this nation (10 grain available)
    {
        let mut production = app.world_mut().resource_mut::<ConnectedProduction>();
        production.totals.insert(
            nation,
            [(ResourceType::Grain, (1, 10))]
                .into_iter()
                .collect(),
        );
    }

    // Set transport capacity but don't allocate any
    {
        let mut capacity = app.world_mut().resource_mut::<TransportCapacity>();
        capacity.nations.insert(
            nation,
            rust_imperialism::economy::transport::CapacitySnapshot {
                total: 20,
                used: 0,
            },
        );
    }

    // Run collection system
    app.add_systems(Update, collect_connected_production);
    app.update();

    // Check stockpile - should be empty because no transport was allocated
    let stockpile = app.world().get::<Stockpile>(nation).unwrap();
    assert_eq!(
        stockpile.get(Good::Grain),
        0,
        "No grain should be collected without transport allocation"
    );

    // Now allocate some transport capacity (5 units for grain)
    {
        let mut allocations = app.world_mut().resource_mut::<TransportAllocations>();
        let nation_alloc = allocations.ensure_nation(nation);
        let slot = nation_alloc.slot_mut(TransportCommodity::Grain);
        slot.requested = 5;
        slot.granted = 5; // Grant the request
    }

    // Run collection again
    app.update();

    // Check stockpile - should have 5 grain (only what was allocated)
    let stockpile = app.world().get::<Stockpile>(nation).unwrap();
    assert_eq!(
        stockpile.get(Good::Grain),
        5,
        "Should collect 5 grain (allocated amount, even though 10 available)"
    );

    // Allocate more capacity (10 units total)
    {
        let mut allocations = app.world_mut().resource_mut::<TransportAllocations>();
        let nation_alloc = allocations.ensure_nation(nation);
        let slot = nation_alloc.slot_mut(TransportCommodity::Grain);
        slot.requested = 10;
        slot.granted = 10;
    }

    // Run collection again
    app.update();

    // Check stockpile - should have 15 grain now (5 from before + 10 new, capped by availability)
    let stockpile = app.world().get::<Stockpile>(nation).unwrap();
    assert_eq!(
        stockpile.get(Good::Grain),
        15, // 5 from first collection + 10 from second (10 is all that was available)
        "Should collect 10 more grain (all remaining available)"
    );
}
