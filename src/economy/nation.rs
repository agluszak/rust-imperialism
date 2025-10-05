use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

/// Unique identifier for a nation (stable across saves)
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NationId(pub u16);

/// Display name for a nation
#[derive(Component, Clone, Debug)]
pub struct Name(pub String);

/// Capital tile position for a nation (used for rail network connectivity)
#[derive(Component, Clone, Copy, Debug)]
pub struct Capital(pub TilePos);

/// Resource pointing to the player's active nation entity
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayerNation(pub Entity);
