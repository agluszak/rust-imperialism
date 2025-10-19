// Core types and structs
pub mod types;
pub use types::{Depot, ImprovementKind, Port, RailConstruction, Rails, Roads, ordered_edge};

// Transport state (capacity, allocations, demand)
pub mod state;
pub use state::{
    AllocationSlot, BASE_TRANSPORT_CAPACITY, CapacitySnapshot, DemandEntry, NationAllocations,
    TransportAllocations, TransportCapacity, TransportCommodity, TransportDemandSnapshot,
};

// Derived metrics and logic
pub mod metrics;
pub use metrics::{
    TransportAdjustAllocation, apply_transport_allocations, transport_capacity, transport_demand,
    transport_slot, update_transport_capacity, update_transport_demand_snapshot,
};

// Messages
pub mod messages;
pub use messages::{PlaceImprovement, RecomputeConnectivity};

// Validation logic
pub mod validation;
pub use validation::{are_adjacent, can_build_rail_on_terrain};

// Construction systems (Logic Layer)
pub mod construction;
pub use construction::advance_rail_construction;

// Connectivity systems (Logic Layer)
pub mod connectivity;
pub use connectivity::{
    build_rail_graph, compute_rail_connectivity, on_depot_added, on_depot_removed, on_port_added,
    on_port_removed,
};

// Input handlers (Input Layer)
pub mod input;
pub use input::apply_improvements;
