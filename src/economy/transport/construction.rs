use bevy::prelude::*;

use crate::economy::transport::messages::RecomputeConnectivity;
use crate::economy::transport::types::{RailConstruction, Rails, ordered_edge};

/// Advance rail construction progress each turn (Logic Layer)
/// Runs during turn processing to decrement construction timers
pub fn advance_rail_construction(
    mut commands: Commands,
    mut constructions: Query<(Entity, &mut RailConstruction)>,
    mut rails: ResMut<Rails>,
    mut connectivity_events: MessageWriter<RecomputeConnectivity>,
) {
    for (entity, mut construction) in constructions.iter_mut() {
        construction.turns_remaining -= 1;

        if construction.turns_remaining == 0 {
            // Construction complete!
            let edge = ordered_edge(construction.from, construction.to);
            rails.0.insert(edge);

            // Trigger connectivity recomputation since topology changed
            connectivity_events.write(RecomputeConnectivity);

            info!(
                "Rail construction complete: ({}, {}) to ({}, {})",
                edge.0.x, edge.0.y, edge.1.x, edge.1.y
            );

            // Remove construction entity
            commands.entity(entity).despawn();
        } else {
            info!(
                "Rail construction: ({}, {}) to ({}, {}) - {} turns remaining",
                construction.from.x,
                construction.from.y,
                construction.to.x,
                construction.to.y,
                construction.turns_remaining
            );
        }
    }
}
