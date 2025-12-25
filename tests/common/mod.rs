use bevy::prelude::NextState;
use rust_imperialism::turn_system::TurnPhase;

/// Helper function to transition between turn phases in tests
/// Encapsulates the double-update pattern needed for state transitions
pub fn transition_to_phase(app: &mut bevy::app::App, phase: TurnPhase) {
    app.world_mut()
        .resource_mut::<NextState<TurnPhase>>()
        .set(phase);
    app.update(); // Apply state transition
    app.update(); // Run systems in the new phase
}
