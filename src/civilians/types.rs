use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet};

use crate::resources::TileResource;

/// Tracks which nations have successfully prospected each mineral tile
#[derive(Resource, Default, Debug)]
pub struct ProspectingKnowledge {
    discoveries: HashMap<Entity, HashSet<Entity>>,
}

impl ProspectingKnowledge {
    /// Record that `nation` has successfully prospected `tile`
    pub fn mark_discovered(&mut self, tile: Entity, nation: Entity) -> bool {
        self.discoveries
            .entry(tile)
            .or_default()
            .insert(nation)
    }

    /// Returns true if `nation` has prospected `tile`
    pub fn is_discovered_by(&self, tile: Entity, nation: Entity) -> bool {
        self.discoveries
            .get(&tile)
            .map_or(false, |nations| nations.contains(&nation))
    }

    /// Forget all prospecting knowledge about `tile`
    pub fn forget_tile(&mut self, tile: Entity) {
        self.discoveries.remove(&tile);
    }

    /// Remove any prospecting knowledge held by `nation`
    pub fn forget_nation(&mut self, nation: Entity) {
        for nations in self.discoveries.values_mut() {
            nations.remove(&nation);
        }
    }
}

/// Resource predicate used to validate whether a civilian can improve a tile
pub type ResourcePredicate = fn(&TileResource) -> bool;

/// Describes the type of multi-turn job a civilian can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum JobType {
    BuildingRail,
    BuildingDepot,
    BuildingPort,
    Mining,
    Drilling,
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
            JobType::Drilling => 3,
            JobType::Prospecting => 1,
            JobType::ImprovingTile => 2,
        }
    }
}

/// How an order is executed once issued
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilianOrderExecution {
    /// Order completes immediately with no persistent job
    Instant,
    /// Order starts a job with a fixed duration
    StartJob(JobType),
}

impl CivilianOrderExecution {
    pub fn job_type(&self) -> Option<JobType> {
        match self {
            CivilianOrderExecution::Instant => None,
            CivilianOrderExecution::StartJob(job_type) => Some(*job_type),
        }
    }
}

/// Descriptor for an order that appears in the civilian UI and is available to logic/AI
#[derive(Debug, Clone, Copy)]
pub struct CivilianOrderDefinition {
    pub label: &'static str,
    pub order: CivilianOrderKind,
    pub execution: CivilianOrderExecution,
}

impl CivilianOrderDefinition {
    /// Returns true if this definition represents the same order variant as `other`
    pub fn matches(&self, other: &CivilianOrderKind) -> bool {
        std::mem::discriminant(&self.order) == std::mem::discriminant(other)
    }
}

/// Static metadata describing how a civilian behaves in the UI and systems
#[derive(Debug, Clone, Copy)]
pub struct CivilianKindDefinition {
    pub display_name: &'static str,
    pub orders: &'static [CivilianOrderDefinition],
    pub resource_predicate: Option<ResourcePredicate>,
    pub improvement_job: Option<JobType>,
    pub show_orders_panel: bool,
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
        const BUILD_DEPOT_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Build Depot",
            order: CivilianOrderKind::BuildDepot,
            execution: CivilianOrderExecution::StartJob(JobType::BuildingDepot),
        };
        const BUILD_PORT_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Build Port",
            order: CivilianOrderKind::BuildPort,
            execution: CivilianOrderExecution::StartJob(JobType::BuildingPort),
        };
        const IMPROVE_TILE_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Improve Tile",
            order: CivilianOrderKind::ImproveTile,
            execution: CivilianOrderExecution::StartJob(JobType::ImprovingTile),
        };
        const MINE_TILE_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Develop Mine",
            order: CivilianOrderKind::Mine,
            execution: CivilianOrderExecution::StartJob(JobType::Mining),
        };
        const DRILL_TILE_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Drill Well",
            order: CivilianOrderKind::ImproveTile,
            execution: CivilianOrderExecution::StartJob(JobType::Drilling),
        };
        const PROSPECT_ORDER: CivilianOrderDefinition = CivilianOrderDefinition {
            label: "Prospect Tile",
            order: CivilianOrderKind::Prospect,
            execution: CivilianOrderExecution::StartJob(JobType::Prospecting),
        };
        const ENGINEER_ORDERS: &[CivilianOrderDefinition] = &[BUILD_DEPOT_ORDER, BUILD_PORT_ORDER];
        const FARMER_ORDERS: &[CivilianOrderDefinition] = &[IMPROVE_TILE_ORDER];
        const RANCHER_ORDERS: &[CivilianOrderDefinition] = &[IMPROVE_TILE_ORDER];
        const FORESTER_ORDERS: &[CivilianOrderDefinition] = &[IMPROVE_TILE_ORDER];
        const MINER_ORDERS: &[CivilianOrderDefinition] = &[MINE_TILE_ORDER];
        const DRILLER_ORDERS: &[CivilianOrderDefinition] = &[DRILL_TILE_ORDER];
        const PROSPECTOR_ORDERS: &[CivilianOrderDefinition] = &[PROSPECT_ORDER];
        const EMPTY_ORDERS: &[CivilianOrderDefinition] = &[];

        const ENGINEER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Engineer",
            orders: ENGINEER_ORDERS,
            resource_predicate: None,
            improvement_job: None,
            show_orders_panel: true,
        };
        const FARMER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Farmer",
            orders: FARMER_ORDERS,
            resource_predicate: Some(TileResource::improvable_by_farmer),
            improvement_job: Some(JobType::ImprovingTile),
            show_orders_panel: false,
        };
        const RANCHER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Rancher",
            orders: RANCHER_ORDERS,
            resource_predicate: Some(TileResource::improvable_by_rancher),
            improvement_job: Some(JobType::ImprovingTile),
            show_orders_panel: false,
        };
        const FORESTER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Forester",
            orders: FORESTER_ORDERS,
            resource_predicate: Some(TileResource::improvable_by_forester),
            improvement_job: Some(JobType::ImprovingTile),
            show_orders_panel: false,
        };
        const MINER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Miner",
            orders: MINER_ORDERS,
            resource_predicate: Some(TileResource::improvable_by_miner),
            improvement_job: Some(JobType::Mining),
            show_orders_panel: false,
        };
        const DRILLER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Driller",
            orders: DRILLER_ORDERS,
            resource_predicate: Some(TileResource::improvable_by_driller),
            improvement_job: Some(JobType::Drilling),
            show_orders_panel: false,
        };
        const PROSPECTOR_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Prospector",
            orders: PROSPECTOR_ORDERS,
            resource_predicate: None,
            improvement_job: None,
            show_orders_panel: false,
        };
        const DEVELOPER_DEFINITION: CivilianKindDefinition = CivilianKindDefinition {
            display_name: "Developer",
            orders: EMPTY_ORDERS,
            resource_predicate: None,
            improvement_job: None,
            show_orders_panel: false,
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
        self.definition().improvement_job.is_some()
    }

    /// Returns true if the civilian should show an orders panel when selected
    pub fn shows_orders_panel(&self) -> bool {
        self.definition().show_orders_panel
    }

    /// Returns the order kind to issue when clicking the civilian's current tile
    pub fn default_tile_action_order(&self) -> Option<CivilianOrderKind> {
        match self {
            CivilianKind::Prospector => Some(CivilianOrderKind::Prospect),
            CivilianKind::Miner => Some(CivilianOrderKind::Mine),
            CivilianKind::Farmer
            | CivilianKind::Rancher
            | CivilianKind::Forester
            | CivilianKind::Driller => Some(CivilianOrderKind::ImproveTile),
            _ => None,
        }
    }

    /// Get the resource predicate used to validate improvements
    pub fn improvement_predicate(&self) -> Option<ResourcePredicate> {
        self.definition().resource_predicate
    }

    /// Get the job type started when issuing an improvement order
    pub fn improvement_job(&self) -> Option<JobType> {
        self.definition().improvement_job
    }

    /// All orders that the civilian exposes to the UI/AI
    pub fn available_orders(&self) -> &'static [CivilianOrderDefinition] {
        self.definition().orders
    }

    /// Return the order definition matching `order`, if available
    pub fn order_definition(
        &self,
        order: &CivilianOrderKind,
    ) -> Option<&'static CivilianOrderDefinition> {
        self.available_orders()
            .iter()
            .find(|definition| definition.matches(order))
    }

    /// Determine if this civilian supports a specific order kind
    pub fn supports_order(&self, order: &CivilianOrderKind) -> bool {
        match order {
            CivilianOrderKind::Move { .. } => true,
            CivilianOrderKind::BuildRail { .. } => *self == CivilianKind::Engineer,
            _ => self.order_definition(order).is_some(),
        }
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
