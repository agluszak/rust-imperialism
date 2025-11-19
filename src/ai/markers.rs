use bevy::prelude::*;

use crate::economy::nation::NationId;

/// Marks a nation entity that should be driven by the AI turn systems.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct AiNation(pub NationId);

/// Marks a civilian unit that is controlled by the AI.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct AiControlledCivilian;
