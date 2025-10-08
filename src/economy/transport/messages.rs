use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use super::types::ImprovementKind;

/// Message to place a transport improvement
#[derive(Message, Debug, Clone, Copy)]
pub struct PlaceImprovement {
    pub a: TilePos,
    pub b: TilePos,
    pub kind: ImprovementKind,
    pub engineer: Option<Entity>, // Engineer entity building this (for tracking construction)
}
