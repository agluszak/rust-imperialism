use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::resources::TileResource;

/// Resource predicate used to validate whether a civilian can improve a tile
pub type ResourcePredicate = fn(&TileResource) -> bool;

/// Descriptor for an action button that appears in the civilian orders UI
#[derive(Debug, Clone, Copy)]
pub struct CivilianActionButton {
    pub label: &'static str,
    pub order: CivilianOrderKind,
}

/// Static metadata describing how a civilian behaves in the UI and systems
#[derive(Debug, Clone, Copy)]
pub struct CivilianKindDefinition {
    pub display_name: &'static str,
    pub action_buttons: &'static [CivilianActionButton],
    pub resource_predicate: Option<ResourcePredicate>,
}

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

impl CivilianKind {
    /// Lookup table for civilian metadata
    pub fn definition(&self) -> &'static CivilianKindDefinition {
        // Static button descriptors reused across definitions
        const IMPROVE_TILE_BUTTON: CivilianActionButton = CivilianActionButton {
            label: "Improve Tile",
            order: CivilianOrderKind::ImproveTile,
        };
        const PROSPECT_BUTTON: CivilianActionButton = CivilianActionButton {
            label: "Prospect Tile",
            order: CivilianOrderKind::Prospect,
        };
        const ENGINEER_ACTIONS: &[CivilianActionButton] = &[
            CivilianActionButton {
                label: "Build Depot",
                order: CivilianOrderKind::BuildDepot,
            },
            CivilianActionButton {
                label: "Build Port",
                order: CivilianOrderKind::BuildPort,
            },
        ];
        const IMPROVER_ACTIONS: &[CivilianActionButton] = &[IMPROVE_TILE_BUTTON];
        const PROSPECTOR_ACTIONS: &[CivilianActionButton] = &[PROSPECT_BUTTON];
        const EMPTY_ACTIONS: &[CivilianActionButton] = &[];

        const ENGINEER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Engineer",
            action_buttons: ENGINEER_ACTIONS,
            resource_predicate: None,
        };
        const FARMER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Farmer",
            action_buttons: IMPROVER_ACTIONS,
            resource_predicate: Some(TileResource::improvable_by_farmer),
        };
        const RANCHER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Rancher",
            action_buttons: IMPROVER_ACTIONS,
            resource_predicate: Some(TileResource::improvable_by_rancher),
        };
        const FORESTER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Forester",
            action_buttons: IMPROVER_ACTIONS,
            resource_predicate: Some(TileResource::improvable_by_forester),
        };
        const MINER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Miner",
            action_buttons: IMPROVER_ACTIONS,
            resource_predicate: Some(TileResource::improvable_by_miner),
        };
        const DRILLER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Driller",
            action_buttons: IMPROVER_ACTIONS,
            resource_predicate: Some(TileResource::improvable_by_driller),
        };
        const PROSPECTOR_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Prospector",
            action_buttons: PROSPECTOR_ACTIONS,
            resource_predicate: None,
        };
        const DEVELOPER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Developer",
            action_buttons: EMPTY_ACTIONS,
            resource_predicate: None,
        };

        match self {
            CivilianKind::Engineer => &ENGINEER_DEFINITION,
            CivilianKind::Farmer => &FARMER_DEFINITION,
            CivilianKind::Rancher => &RANCHER_DEFINITION,
            CivilianKind::Forester => &FORESTER_DEFINITION,
            CivilianKind::Miner => &MINER_DEFINITION,
            CivilianKind::Driller => &DRILLER_DEFINITION,
            CivilianKind::Prospector => &PROSPECTOR_DEFINITION,
            CivilianKind::Developer => &DEVELOPER_DEFINITION,
        }
    }

    /// Returns true if the civilian can improve tile resources
    pub fn supports_improvements(&self) -> bool {
        self.definition().resource_predicate.is_some()
    }

    /// Get the resource predicate used to validate improvements
    pub fn improvement_predicate(&self) -> Option<ResourcePredicate> {
        self.definition().resource_predicate
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
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
