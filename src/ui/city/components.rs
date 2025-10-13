use crate::civilians::CivilianKind;
use bevy::prelude::*;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

// ============ HUD Border Components ============

/// Left border: Labor pool panel
#[derive(Component)]
pub struct LaborPoolPanel;

/// Display for available labor (updates live)
#[derive(Component)]
pub struct AvailableLaborDisplay;

/// Display for workforce counts (untrained/trained/expert)
#[derive(Component)]
pub struct WorkforceCountDisplay;

/// Right border: Food demand panel
#[derive(Component)]
pub struct FoodDemandPanel;

/// Display for food demand by type
#[derive(Component)]
pub struct FoodDemandDisplay;

/// Top center: Compact warehouse HUD
#[derive(Component)]
pub struct WarehouseHUD;

/// Display for warehouse stock (updates live)
#[derive(Component)]
pub struct WarehouseStockDisplay;

// ============ Building Grid Components ============

/// Marker for the building grid container
#[derive(Component)]
pub struct BuildingGrid;

/// Button for a building (either built or available to build)
#[derive(Component, Clone)]
pub struct BuildingButton {
    pub building_entity: Option<Entity>, // None if not built yet
    pub building_kind: crate::economy::production::BuildingKind,
}

/// Marker for hire civilian buttons
#[derive(Component)]
pub struct HireCivilianButton(pub CivilianKind);

/// Marker for production choice buttons
#[derive(Component)]
pub struct ProductionChoiceButton {
    pub building_entity: Entity,
    pub choice: crate::economy::production::ProductionChoice,
}

/// Message to hire a civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct HireCivilian {
    pub kind: CivilianKind,
}

/// Marker for production dialog labor display
#[derive(Component)]
pub struct ProductionLaborDisplay {
    pub building_entity: Entity,
}

/// Marker for Capitol dialog requirement displays
#[derive(Component)]
pub struct CapitolRequirementDisplay {
    pub good: crate::economy::Good,
}

/// Marker for Capitol dialog capacity display
#[derive(Component)]
pub struct CapitolCapacityDisplay;

/// Marker for Trade School workforce displays
#[derive(Component)]
pub struct TradeSchoolWorkforceDisplay;

/// Marker for Trade School paper display
#[derive(Component)]
pub struct TradeSchoolPaperDisplay;

/// Marker for recruit workers button
#[derive(Component)]
pub struct RecruitWorkersButton {
    pub count: u32,
}

/// Marker for train worker button
#[derive(Component)]
pub struct TrainWorkerButton {
    pub from_skill: crate::economy::WorkerSkill,
}
