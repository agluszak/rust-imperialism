// Core types and structs
pub mod types;
pub use types::{Depot, ImprovementKind, Port, RailConstruction, Rails, Roads, ordered_edge};

// Messages
pub mod messages;
pub use messages::PlaceImprovement;

// Validation logic
pub mod validation;
pub use validation::{are_adjacent, can_build_rail_on_terrain};

// Construction systems (Logic Layer)
pub mod construction;
pub use construction::advance_rail_construction;

// Connectivity systems (Logic Layer)
pub mod connectivity;
pub use connectivity::{build_rail_graph, compute_rail_connectivity};

// Input handlers (Input Layer)
pub mod input;
pub use input::apply_improvements;
