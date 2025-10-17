use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

/// Type of civilian unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CivilianKind {
    Prospector, // Reveals minerals (coal/iron/gold/gems/oil)
    Miner,      // Opens & upgrades mines
    Farmer,     // Improves grain/fruit/cotton
    Rancher,    // Improves wool/livestock
    Forester,   // Improves timber
    Driller,    // Improves oil
    Engineer,   // Builds rails, depots, ports, fortifications
    Developer,  // Works in Minor Nations
}

/// Civilian unit component
#[derive(Component, Debug, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Civilian {
    pub kind: CivilianKind,
    pub position: TilePos,
    pub owner: Entity, // Nation entity that owns this unit
    pub selected: bool,
    pub has_moved: bool, // True if unit has used its action this turn
}

/// Pending order for a civilian unit
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CivilianOrder {
    pub target: CivilianOrderKind,
}

/// Ongoing multi-turn job for a civilian
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct CivilianJob {
    pub job_type: JobType,
    pub turns_remaining: u32,
    pub target: TilePos, // Where the job is happening
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum JobType {
    BuildingRail,
    BuildingDepot,
    BuildingPort,
    Mining,
    Prospecting,
    ImprovingTile,
}

impl JobType {
    /// Get the number of turns required for this job type
    pub fn duration(&self) -> u32 {
        match self {
            JobType::BuildingRail => 3,
            JobType::BuildingDepot => 2,
            JobType::BuildingPort => 2,
            JobType::Mining => 2,
            JobType::Prospecting => 1,
            JobType::ImprovingTile => 2,
        }
    }
}

/// Visual marker for civilian unit sprites
#[derive(Component)]
pub struct CivilianVisual(pub Entity); // Points to the Civilian entity

/// Stores the previous position of a civilian before they moved/acted
/// Used to allow "undo" of moves at any time before the job completes
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct PreviousPosition(pub TilePos);

/// Tracks which turn an action was taken on
/// Used to determine if resources should be refunded when rescinding
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct ActionTurn(pub u32);

#[derive(Debug, Clone, Copy, Reflect)]
pub enum CivilianOrderKind {
    BuildRail { to: TilePos }, // Build rail to adjacent tile
    BuildDepot,                // Build depot at current position
    BuildPort,                 // Build port at current position
    Move { to: TilePos },      // Move to target tile
    Prospect,                  // Reveal minerals at current tile (Prospector)
    Mine,                      // Upgrade mine at current tile (Miner)
    ImproveTile,               // Improve resource at current tile (Farmer/Rancher/Forester/Driller)
    BuildFarm,                 // Build farm on grain/fruit/cotton tile (Farmer)
    BuildOrchard,              // Build orchard on fruit tile (Farmer)
}

impl MapEntities for Civilian {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}
