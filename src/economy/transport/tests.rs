use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::HashMap;

use crate::economy::allocation::Allocations;
use crate::economy::production::{Buildings, ConnectedProduction};
use crate::economy::transport::{
    apply_transport_allocations, update_transport_capacity, update_transport_demand_snapshot,
    TransportAllocations, TransportCapacity, TransportCommodity, TransportDemandSnapshot,
};
use crate::economy::workforce::Workforce;
use crate::resources::ResourceType;
use super::types::{Depot, Port};

#[test]
fn capacity_totals_respect_connected_improvements() {
    let mut app = App::new();
    app.init_resource::<TransportCapacity>();

    let nation = app.world_mut().spawn_empty().id();

    app.world_mut().spawn(Depot {
        position: TilePos { x: 0, y: 0 },
        owner: nation,
        connected: true,
    });

    app.world_mut().spawn(Depot {
        position: TilePos { x: 1, y: 1 },
        owner: nation,
        connected: false,
    });

    app.world_mut().spawn(Port {
        position: TilePos { x: 2, y: 2 },
        owner: nation,
        connected: true,
        is_river: false,
    });

    app.add_systems(Update, update_transport_capacity);
    app.update();

    let capacity = app.world().resource::<TransportCapacity>();
    let snapshot = capacity.snapshot(nation);
    // Depot contributes 6, port contributes 8 => 14 total
    assert_eq!(snapshot.total, 14);
    assert_eq!(snapshot.used, 0);
}

#[test]
fn allocation_granted_values_clamp_to_total_capacity() {
    let mut app = App::new();
    app.init_resource::<TransportCapacity>();
    app.init_resource::<TransportAllocations>();
    app.add_systems(Update, apply_transport_allocations);

    let nation = app.world_mut().spawn_empty().id();

    {
        let mut capacity = app.world_mut().resource_mut::<TransportCapacity>();
        let snapshot = capacity.snapshot_mut(nation);
        snapshot.total = 5;
        snapshot.used = 0;
    }

    {
        let mut allocations = app
            .world_mut()
            .resource_mut::<TransportAllocations>();
        let nation_alloc = allocations.ensure_nation(nation);
        nation_alloc
            .slot_mut(TransportCommodity::Grain)
            .requested = 4;
        nation_alloc
            .slot_mut(TransportCommodity::Coal)
            .requested = 4;
    }

    app.update();

    let allocations = app.world().resource::<TransportAllocations>();
    let grain_slot = allocations.slot(nation, TransportCommodity::Grain);
    let coal_slot = allocations.slot(nation, TransportCommodity::Coal);
    assert_eq!(grain_slot.granted, 4);
    assert_eq!(coal_slot.granted, 1);

    let capacity = app.world().resource::<TransportCapacity>();
    let snapshot = capacity.snapshot(nation);
    assert_eq!(snapshot.used, 5);
}

#[test]
fn demand_snapshot_collects_supply_and_worker_demand() {
    let mut app = App::new();
    app.init_resource::<TransportDemandSnapshot>();

    let nation = app
        .world_mut()
        .spawn((Allocations::default(), Buildings::with_all_initial(), Workforce::new()))
        .id();

    {
        let mut workforce = app.world_mut().get_mut::<Workforce>(nation).unwrap();
        workforce.add_untrained(3);
    }

    let mut connected = ConnectedProduction::default();
    let mut resource_map = HashMap::new();
    resource_map.insert(ResourceType::Grain, (1, 6));
    resource_map.insert(ResourceType::Coal, (1, 3));
    connected.0.insert(nation, resource_map);
    app.insert_resource(connected);

    app.add_systems(Update, update_transport_demand_snapshot);
    app.update();

    let snapshot = app.world().resource::<TransportDemandSnapshot>();
    let nation_entries = snapshot
        .nations
        .get(&nation)
        .expect("demand entry for nation");

    let grain = nation_entries
        .get(&TransportCommodity::Grain)
        .expect("grain entry");
    assert_eq!(grain.supply, 6);
    assert_eq!(grain.demand, 1);

    let fruit = nation_entries
        .get(&TransportCommodity::Fruit)
        .expect("fruit entry");
    assert_eq!(fruit.demand, 1);

    let meat = nation_entries
        .get(&TransportCommodity::Meat)
        .expect("meat entry");
    assert_eq!(meat.demand, 1);

    let coal = nation_entries
        .get(&TransportCommodity::Coal)
        .expect("coal entry");
    assert_eq!(coal.supply, 3);
}
