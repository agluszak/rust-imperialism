use std::path::PathBuf;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_save::prelude::*;

use crate::civilians::types::{
    ActionTurn, Civilian, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind, JobType,
    PreviousPosition,
};
use crate::economy::allocation::Allocations;
use crate::economy::goods::Good;
use crate::economy::nation::{Capital, Name, NationColor, NationId, PlayerNation};
use crate::economy::production::{
    Building, BuildingKind, Buildings, ProductionChoice, ProductionSettings,
};
use crate::economy::reservation::{ReservationSystem, ResourcePool};
use crate::economy::stockpile::Stockpile;
use crate::economy::technology::{Technologies, Technology};
use crate::economy::transport::{Depot, Port, RailConstruction, Rails, Roads};
use crate::economy::treasury::Treasury;
use crate::economy::workforce::types::{Worker, WorkerHealth, WorkerSkill, Workforce};
use crate::economy::{Calendar, RecruitmentCapacity, RecruitmentQueue, Season, TrainingQueue};
use crate::province::{City, Province, ProvinceId, TileProvince};
use crate::province_setup::ProvincesGenerated;
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::menu::AppState;

/// Plugin that wires the moonshine save/load pipeline into the game.
pub struct GameSavePlugin;

/// Default save settings (currently only the fallback save path).
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct SaveSettings {
    /// Default filesystem path used when requests do not provide one.
    pub default_path: PathBuf,
}

impl Default for SaveSettings {
    fn default() -> Self {
        Self {
            default_path: PathBuf::from("saves/autosave.ron"),
        }
    }
}

/// Request to write the current game state to disk.
#[derive(Message, Clone)]
pub struct SaveGameRequest {
    pub path: Option<PathBuf>,
}

/// Request to load a saved game from disk.
#[derive(Message, Clone)]
pub struct LoadGameRequest {
    pub path: Option<PathBuf>,
}

/// Notification emitted after a successful save operation.
#[derive(Message, Clone)]
pub struct SaveGameCompleted {
    pub path: PathBuf,
}

/// Notification emitted after a successful load operation.
#[derive(Message, Clone)]
pub struct LoadGameCompleted {
    pub path: PathBuf,
}

#[derive(Resource, Default)]
struct PendingSave {
    path: Option<PathBuf>,
}

#[derive(Resource, Default)]
struct PendingLoad {
    path: Option<PathBuf>,
}

impl Plugin for GameSavePlugin {
    fn build(&self, app: &mut App) {
        register_reflect_types(app);

        app.init_resource::<SaveSettings>()
            .init_resource::<PendingSave>()
            .init_resource::<PendingLoad>()
            .add_message::<SaveGameRequest>()
            .add_message::<LoadGameRequest>()
            .add_message::<SaveGameCompleted>()
            .add_message::<LoadGameCompleted>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_observer(emit_save_completion)
            .add_observer(emit_load_completion)
            .add_observer(rebuild_runtime_state_after_load)
            .add_systems(
                Update,
                (
                    process_save_requests,
                    process_load_requests,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn register_reflect_types(app: &mut App) {
    app.register_type::<TilePos>()
        .register_type::<Good>()
        .register_type::<ResourcePool>()
        .register_type::<Stockpile>()
        .register_type::<Technology>()
        .register_type::<Technologies>()
        .register_type::<NationId>()
        .register_type::<Name>()
        .register_type::<NationColor>()
        .register_type::<Capital>()
        .register_type::<Treasury>()
        .register_type::<Season>()
        .register_type::<Calendar>()
        .register_type::<TurnPhase>()
        .register_type::<TurnSystem>()
        .register_type::<ProvinceId>()
        .register_type::<Province>()
        .register_type::<City>()
        .register_type::<TileProvince>()
        .register_type::<CivilianKind>()
        .register_type::<Civilian>()
        .register_type::<CivilianOrder>()
        .register_type::<CivilianOrderKind>()
        .register_type::<CivilianJob>()
        .register_type::<JobType>()
        .register_type::<PreviousPosition>()
        .register_type::<ActionTurn>()
        .register_type::<ProductionChoice>()
        .register_type::<ProductionSettings>()
        .register_type::<BuildingKind>()
        .register_type::<Building>()
        .register_type::<Buildings>()
        .register_type::<WorkerSkill>()
        .register_type::<WorkerHealth>()
        .register_type::<Worker>()
        .register_type::<Workforce>()
        .register_type::<RecruitmentCapacity>()
        .register_type::<RecruitmentQueue>()
        .register_type::<TrainingQueue>()
        .register_type::<Roads>()
        .register_type::<Rails>()
        .register_type::<Depot>()
        .register_type::<Port>()
        .register_type::<RailConstruction>()
        .register_type::<ProvincesGenerated>();
}

fn process_save_requests(
    mut commands: Commands,
    mut requests: MessageReader<SaveGameRequest>,
    settings: Res<SaveSettings>,
    mut pending: ResMut<PendingSave>,
) {
    for request in requests.read() {
        let path = request
            .path
            .clone()
            .unwrap_or_else(|| settings.default_path.clone());

        let event = SaveWorld::default_into_file(path.clone())
            .include_resource::<Calendar>()
            .include_resource::<TurnSystem>()
            .include_resource::<Roads>()
            .include_resource::<Rails>()
            .include_resource::<ProvincesGenerated>()
            .exclude_component::<Allocations>()
            .exclude_component::<ReservationSystem>();

        commands.trigger_save(event);
        pending.path = Some(path);
    }
}

fn process_load_requests(
    mut commands: Commands,
    mut requests: MessageReader<LoadGameRequest>,
    settings: Res<SaveSettings>,
    mut pending: ResMut<PendingLoad>,
) {
    for request in requests.read() {
        let path = request
            .path
            .clone()
            .unwrap_or_else(|| settings.default_path.clone());

        commands.trigger_load(LoadWorld::default_from_file(path.clone()));
        pending.path = Some(path);
    }
}

fn emit_save_completion(
    _: On<Saved>,
    mut pending: ResMut<PendingSave>,
    mut completed: MessageWriter<SaveGameCompleted>,
) {
    if let Some(path) = pending.path.take() {
        completed.write(SaveGameCompleted { path });
    }
}

fn emit_load_completion(
    _: On<Loaded>,
    mut pending: ResMut<PendingLoad>,
    mut completed: MessageWriter<LoadGameCompleted>,
) {
    if let Some(path) = pending.path.take() {
        completed.write(LoadGameCompleted { path });
    }
}

fn rebuild_runtime_state_after_load(
    _: On<Loaded>,
    mut commands: Commands,
    mut player_nation: Option<ResMut<PlayerNation>>,
    nations: Query<(
        Entity,
        &NationId,
        Option<&Name>,
        Option<&Allocations>,
        Option<&ReservationSystem>,
    )>,
) {
    let mut player_entity = None;

    for (entity, nation_id, name, allocations, reservations) in nations.iter() {
        if allocations.is_none() {
            commands.entity(entity).insert(Allocations::default());
        }

        if reservations.is_none() {
            commands.entity(entity).insert(ReservationSystem::default());
        }

        if nation_id.0 == 1
            || name
                .map(|Name(label)| label.as_str() == "Player")
                .unwrap_or(false)
        {
            player_entity = Some(entity);
        }
    }

    if let Some(entity) = player_entity {
        if let Some(existing) = player_nation.as_mut() {
            existing.0 = entity;
        } else {
            commands.insert_resource(PlayerNation(entity));
        }
    }
}
