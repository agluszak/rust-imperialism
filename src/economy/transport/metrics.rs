use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::{
    allocation::Allocations,
    goods::Good,
    production::{BuildingKind, Buildings, ProductionChoice},
    transport::{
        AllocationSlot, CapacitySnapshot, DemandEntry, TransportAllocations, TransportCapacity,
        TransportCommodity, TransportDemandSnapshot,
    },
    workforce::Workforce,
};

use super::types::{Depot, Port};

/// Base capacity contributed by each connected depot.
const DEPOT_CAPACITY: u32 = 6;
/// Base capacity contributed by each connected port.
const PORT_CAPACITY: u32 = 8;

/// Recalculate transport capacity totals from connected depots and ports.
pub fn update_transport_capacity(
    mut capacity: ResMut<TransportCapacity>,
    depots: Query<&Depot>,
    ports: Query<&Port>,
) {
    let mut totals: HashMap<Entity, CapacitySnapshot> = HashMap::new();

    for depot in depots.iter().filter(|d| d.connected) {
        let entry = totals.entry(depot.owner).or_default();
        entry.total += DEPOT_CAPACITY;
    }

    for port in ports.iter().filter(|p| p.connected) {
        let entry = totals.entry(port.owner).or_default();
        entry.total += PORT_CAPACITY;
    }

    capacity
        .nations
        .retain(|nation, _| totals.contains_key(nation));

    for (nation, totals_snapshot) in totals {
        let snapshot = capacity.snapshot_mut(nation);
        snapshot.total = totals_snapshot.total;
        snapshot.used = snapshot.used.min(snapshot.total);
    }
}

/// Message emitted from UI sliders requesting a new allocation level.
#[derive(Message, Debug, Clone, Copy)]
pub struct TransportAdjustAllocation {
    pub nation: Entity,
    pub commodity: TransportCommodity,
    pub requested: u32,
}

/// Apply allocation adjustments while respecting total capacity.
pub fn apply_transport_allocations(
    mut capacity: ResMut<TransportCapacity>,
    mut allocations: ResMut<TransportAllocations>,
    mut requests: MessageReader<TransportAdjustAllocation>,
) {
    for request in requests.read() {
        let nation_alloc = allocations.ensure_nation(request.nation);
        let slot = nation_alloc.slot_mut(request.commodity);
        slot.requested = request.requested;
    }

    // Recompute granted totals per nation.
    for (nation, nation_alloc) in allocations.nations.iter_mut() {
        let mut remaining = capacity.snapshot(*nation).total;
        for commodity in TransportCommodity::ORDERED.iter() {
            if let Some(slot) = nation_alloc.commodities.get_mut(commodity) {
                let granted = slot.requested.min(remaining);
                slot.granted = granted;
                remaining = remaining.saturating_sub(granted);
            }
        }
        capacity.snapshot_mut(*nation).used = capacity.snapshot(*nation).total - remaining;
    }
}

/// Helper describing input requirements for one unit of production.
fn inputs_for_output(
    kind: BuildingKind,
    choice: ProductionChoice,
    _output: Good,
) -> Vec<(Good, u32)> {
    match kind {
        BuildingKind::TextileMill => match choice {
            ProductionChoice::UseCotton => vec![(Good::Cotton, 2)],
            ProductionChoice::UseWool => vec![(Good::Wool, 2)],
            _ => vec![(Good::Cotton, 2)],
        },
        BuildingKind::LumberMill => vec![(Good::Timber, 2)],
        BuildingKind::SteelMill => vec![(Good::Iron, 1), (Good::Coal, 1)],
        BuildingKind::FoodProcessingCenter => {
            let meat = match choice {
                ProductionChoice::UseFish => Good::Fish,
                _ => Good::Livestock,
            };
            // Output is in canned food units (2 per batch)
            vec![(Good::Grain, 2), (Good::Fruit, 1), (meat, 1)]
        }
        BuildingKind::Capitol | BuildingKind::TradeSchool | BuildingKind::PowerPlant => vec![],
    }
}

/// Update supply/demand snapshot for transport UI.
pub fn update_transport_demand_snapshot(
    connected_production: Res<crate::economy::production::ConnectedProduction>,
    workforces: Query<(Entity, &Workforce)>,
    allocations: Query<(Entity, &Allocations, Option<&Buildings>)>,
    mut snapshot: ResMut<TransportDemandSnapshot>,
) {
    snapshot.nations.clear();

    // Supply from connected production
    for (nation, resources) in connected_production.0.iter() {
        let entries = snapshot.nations.entry(*nation).or_default();
        for commodity in TransportCommodity::ORDERED.iter() {
            let mut supply = 0u32;
            for resource_type in commodity.resource_types() {
                if let Some((_, output)) = resources.get(resource_type) {
                    supply += *output;
                }
            }
            if supply > 0 {
                entries
                    .entry(*commodity)
                    .or_default()
                    .supply = supply;
            }
        }
    }

    // Demand from workforce (food)
    for (entity, workforce) in workforces.iter() {
        let entries = snapshot.nations.entry(entity).or_default();
        for (index, _worker) in workforce.workers.iter().enumerate() {
            let commodity = match index % 3 {
                0 => TransportCommodity::Grain,
                1 => TransportCommodity::Fruit,
                _ => TransportCommodity::Meat,
            };
            entries
                .entry(commodity)
                .or_default()
                .demand += 1;
        }
    }

    // Demand from production allocations
    for (nation, alloc, maybe_buildings) in allocations.iter() {
        let entries = snapshot.nations.entry(nation).or_default();

        for ((_, output_good), reservations) in alloc.production.iter() {
            let Some(buildings) = maybe_buildings else {
                continue;
            };

            let building_kind = match output_good {
                Good::Fabric | Good::Cloth => BuildingKind::TextileMill,
                Good::Paper | Good::Lumber => BuildingKind::LumberMill,
                Good::Steel => BuildingKind::SteelMill,
                Good::CannedFood => BuildingKind::FoodProcessingCenter,
                _ => continue,
            };

            let building = buildings.get(building_kind);
            if building.is_none() {
                continue;
            }

            let choice = match building_kind {
                BuildingKind::TextileMill => ProductionChoice::UseCotton,
                BuildingKind::FoodProcessingCenter => ProductionChoice::UseLivestock,
                _ => ProductionChoice::UseCotton,
            };

            let inputs = inputs_for_output(building_kind, choice, *output_good);
            let units = reservations.len() as u32;

            for (good, amount_per_unit) in inputs {
                if let Some(commodity) = TransportCommodity::from_good(good) {
                    entries
                        .entry(commodity)
                        .or_default()
                        .demand += amount_per_unit * units;
                }
            }
        }
    }
}

/// Provide quick access to allocation slots for UI rendering.
pub fn transport_slot(
    allocations: &TransportAllocations,
    nation: Entity,
    commodity: TransportCommodity,
) -> AllocationSlot {
    allocations.slot(nation, commodity)
}

/// Provide quick access to demand entries for UI rendering.
pub fn transport_demand(
    snapshot: &TransportDemandSnapshot,
    nation: Entity,
    commodity: TransportCommodity,
) -> DemandEntry {
    snapshot
        .nations
        .get(&nation)
        .and_then(|map| map.get(&commodity))
        .copied()
        .unwrap_or_default()
}

/// Provide quick access to capacity snapshot for UI rendering.
pub fn transport_capacity(capacity: &TransportCapacity, nation: Entity) -> CapacitySnapshot {
    capacity.snapshot(nation)
}
