use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_save::prelude::Save;
use std::collections::HashSet;

/// Type of transport improvement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ImprovementKind {
    Road,  // Early-game low-capacity transport
    Rail,  // High-capacity transport network
    Depot, // Gathers resources from tile + 8 neighbors
    Port,  // Coastal/river gathering point
}

/// Marker component for depots that gather resources
#[derive(Component, Debug, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save)]
pub struct Depot {
    pub position: TilePos,
    pub owner: Entity,   // Nation entity that owns this depot
    pub connected: bool, // Whether this depot has a rail path to owner's capital
}

/// Marker component for ports (coastal or river)
#[derive(Component, Debug, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save)]
pub struct Port {
    pub position: TilePos,
    pub owner: Entity, // Nation entity that owns this port
    pub connected: bool,
    pub is_river: bool,
}

/// Roads are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct Roads(pub HashSet<(TilePos, TilePos)>);

/// Rails are stored as ordered, undirected edge pairs between adjacent tiles
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct Rails(pub HashSet<(TilePos, TilePos)>);

/// Component tracking rail construction in progress (takes 2 turns to complete)
#[derive(Component, Debug, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save)]
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

impl MapEntities for Depot {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

impl MapEntities for Port {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

impl MapEntities for RailConstruction {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
        self.engineer = mapper.get_mapped(self.engineer);
    }
}
