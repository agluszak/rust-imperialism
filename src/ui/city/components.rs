use crate::civilians::CivilianKind;
use bevy::prelude::*;

/// Marker for the root of the City UI screen
#[derive(Component)]
pub struct CityScreen;

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
