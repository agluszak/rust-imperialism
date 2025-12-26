use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use moonshine_save::prelude::Save;

/// Unique identifier for a ship (stable across saves)
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Hash, Reflect)]
#[reflect(Component)]
#[require(Save)]
pub struct ShipId(pub u32);

/// Resource to generate unique ShipIds
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct NextShipId(u32);

impl NextShipId {
    /// Generate a new unique ShipId
    pub fn next_id(&mut self) -> ShipId {
        let id = ShipId(self.0);
        self.0 += 1;
        id
    }
}

/// Type of merchant ship (based on manual: Trader, Indiaman, Steamship, Clipper, Freighter)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ShipKind {
    /// Basic merchant ship (Trader)
    Trader,
    /// Intermediate merchant ship (Indiaman)
    Indiaman,
    /// Steam-powered merchant ship (Steamship)
    Steamship,
    /// Fast merchant ship (Clipper)
    Clipper,
    /// Large capacity merchant ship (Freighter)
    Freighter,
}

impl ShipKind {
    /// Get the cargo capacity for this ship type
    pub fn cargo_capacity(self) -> u32 {
        match self {
            ShipKind::Trader => 1,
            ShipKind::Indiaman => 2,
            ShipKind::Steamship => 2,
            ShipKind::Clipper => 2,
            ShipKind::Freighter => 3,
        }
    }
}

impl Default for ShipKind {
    fn default() -> Self {
        ShipKind::Trader
    }
}

/// Merchant ship entity component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save, Name)]
pub struct Ship {
    pub kind: ShipKind,
    #[entities]
    pub owner: Entity, // Nation entity that owns this ship (remapped via MapEntities)
    pub ship_id: ShipId,
    pub has_moved: bool, // True if ship has been used for trade this turn
}

impl MapEntities for Ship {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

impl Ship {
    /// Create a new ship
    pub fn new(kind: ShipKind, owner: Entity, ship_id: ShipId) -> Self {
        Self {
            kind,
            owner,
            ship_id,
            has_moved: false,
        }
    }

    /// Get the cargo capacity of this ship
    pub fn cargo_capacity(&self) -> u32 {
        self.kind.cargo_capacity()
    }
}
