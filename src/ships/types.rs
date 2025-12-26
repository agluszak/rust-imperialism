use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use moonshine_save::prelude::Save;

/// Type of merchant ship (based on manual: Trader, Indiaman, Steamship, Clipper, Freighter)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum ShipKind {
    /// Basic merchant ship (Trader)
    #[default]
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

/// Merchant ship entity component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save, Name)]
pub struct Ship {
    pub kind: ShipKind,
    #[entities]
    pub owner: Entity, // Nation entity that owns this ship (remapped via MapEntities)
    pub has_moved: bool, // True if ship has been used for trade this turn
}

impl MapEntities for Ship {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

impl Ship {
    /// Create a new ship
    pub fn new(kind: ShipKind, owner: Entity) -> Self {
        Self {
            kind,
            owner,
            has_moved: false,
        }
    }

    /// Get the cargo capacity of this ship
    pub fn cargo_capacity(&self) -> u32 {
        self.kind.cargo_capacity()
    }
}
