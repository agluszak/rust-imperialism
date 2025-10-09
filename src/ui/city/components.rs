use crate::civilians::CivilianKind;
use bevy::prelude::*;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

// ============ NEW: HUD Border Components ============

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

// ============ END HUD Components ============

// ============ NEW: Building Grid Components ============

/// Marker for the building grid container
#[derive(Component)]
pub struct BuildingGrid;

/// Button for a building (either built or available to build)
#[derive(Component, Clone)]
pub struct BuildingButton {
    pub building_entity: Option<Entity>, // None if not built yet
    pub building_kind: crate::economy::production::BuildingKind,
}

// ============ END Building Grid Components ============

/// Marker for hire civilian buttons
#[derive(Component)]
pub struct HireCivilianButton(pub CivilianKind);

/// Marker for building panels (dynamically created)
#[derive(Component)]
pub struct BuildingPanel;

/// Marker for production choice buttons
#[derive(Component)]
pub struct ProductionChoiceButton {
    pub building_entity: Entity,
    pub choice: crate::economy::production::ProductionChoice,
}

/// Marker for increase/decrease production buttons
#[derive(Component)]
pub struct AdjustProductionButton {
    pub building_entity: Entity,
    pub delta: i32, // +1 or -1
}

/// Message to hire a civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct HireCivilian {
    pub kind: CivilianKind,
}

/// Message to change production settings
#[derive(Message, Debug, Clone, Copy)]
pub struct ChangeProductionSettings {
    pub building_entity: Entity,
    pub new_choice: Option<crate::economy::production::ProductionChoice>,
    pub target_delta: Option<i32>,
}

/// Marker for production dialog target output display (the number between +/- buttons)
#[derive(Component)]
pub struct ProductionTargetDisplay {
    pub building_entity: Entity,
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

/// Marker for the workforce panel
#[derive(Component)]
pub struct WorkforcePanel;

/// Marker for workforce counts text (updates dynamically)
#[derive(Component)]
pub struct WorkforceCountsText;

/// Marker for available labor text (updates dynamically)
#[derive(Component)]
pub struct AvailableLaborText;

/// Marker for stockpile food text
#[derive(Component)]
pub struct StockpileFoodText;

/// Marker for stockpile materials text
#[derive(Component)]
pub struct StockpileMaterialsText;

/// Marker for stockpile goods text
#[derive(Component)]
pub struct StockpileGoodsText;

// ============ NEW: Allocation UI Components ============

/// Marker for recruitment allocation slider/stepper
#[derive(Component)]
pub struct RecruitmentAllocationStepper;

/// Button to adjust recruitment allocation
#[derive(Component)]
pub struct AdjustRecruitmentButton {
    pub delta: i32, // +1, +5, -1, -5
}

/// Display for recruitment allocation value
#[derive(Component)]
pub struct RecruitmentAllocationDisplay;

/// Allocation bar for a specific good in recruitment
#[derive(Component)]
pub struct RecruitmentAllocationBar {
    pub good: crate::economy::Good,
}

/// Button to adjust training allocation
#[derive(Component)]
pub struct AdjustTrainingButton {
    pub from_skill: crate::economy::WorkerSkill,
    pub delta: i32, // +1, -1
}

/// Display for training allocation value
#[derive(Component)]
pub struct TrainingAllocationDisplay {
    pub from_skill: crate::economy::WorkerSkill,
}

/// Allocation bar for training (paper)
#[derive(Component)]
pub struct TrainingAllocationBar {
    pub from_skill: crate::economy::WorkerSkill,
}
