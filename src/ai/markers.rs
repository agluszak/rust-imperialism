use bevy::prelude::*;

/// Marks a nation entity that should be driven by the AI turn systems.
#[derive(Component, Debug, Default)]
pub struct AiNation;

/// Marks a civilian unit that is controlled by the AI.
#[derive(Component, Debug, Default)]
pub struct AiControlledCivilian;
