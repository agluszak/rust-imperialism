use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::HashSet;

/// Type of transport improvement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementKind {
    Road,  // Early-game low-capacity transport
    Rail,  // High-capacity transport network
    Depot, // Gathers resources from tile + 8 neighbors
    Port,  // Coastal/river gathering point
}

/// Marker component for depots that gather resources
#[derive(Component, Debug)]
pub struct Depot {
    pub position: TilePos,
    pub owner: Entity,   // Nation entity that owns this depot
    pub connected: bool, // Whether this depot has a rail path to owner's capital
}

/// Marker component for ports (coastal or river)
#[derive(Component, Debug)]
pub struct Port {
    pub position: TilePos,
    pub owner: Entity, // Nation entity that owns this port
    pub connected: bool,
    pub is_river: bool,
}

/// Roads are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug)]
pub struct Roads(pub HashSet<(TilePos, TilePos)>);

/// Rails are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug)]
pub struct Rails(pub HashSet<(TilePos, TilePos)>);

/// Component tracking rail construction in progress (takes 3 turns to complete)
#[derive(Component, Debug)]
pub struct RailConstruction {
    pub from: TilePos,
    pub to: TilePos,
    pub turns_remaining: u32,
    pub owner: Entity,    // Nation that started construction
    pub engineer: Entity, // Engineer entity that is building this
}

/// Helper function to create an ordered edge for consistent storage
pub fn ordered_edge(a: TilePos, b: TilePos) -> (TilePos, TilePos) {
    if (a.x, a.y) <= (b.x, b.y) {
        (a, b)
    } else {
        (b, a)
    }
}
