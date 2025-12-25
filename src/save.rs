use std::path::PathBuf;

use bevy::prelude::*;
use moonshine_save::prelude::*;

use crate::ai::markers::{AiControlledCivilian, AiNation};
use crate::civilians::{
    ActionTurn, Civilian, CivilianId, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind,
    JobType, NextCivilianId, PreviousPosition, ProspectingKnowledge,
};
use crate::economy::allocation::Allocations;
use crate::economy::goods::Good;
use crate::economy::nation::{Capital, Nation, NationColor, PlayerNation};
use crate::economy::production::{
    Building, BuildingKind, Buildings, ProductionChoice, ProductionSettings,
};
use crate::economy::reservation::{ReservationSystem, ResourcePool};
use crate::economy::stockpile::Stockpile;
use crate::economy::technology::{Technologies, Technology};
use crate::economy::transport::{Depot, ImprovementKind, Port, RailConstruction, Rails, Roads};
use crate::economy::treasury::Treasury;
use crate::economy::workforce::{
    RecruitmentCapacity, RecruitmentQueue, TrainingQueue, Worker, WorkerHealth, WorkerSkill,
    Workforce,
};
use crate::economy::{Calendar, Season};
use crate::map::province::{City, Province, ProvinceId};
use crate::map::province_setup::ProvincesGenerated;
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
                (process_save_requests, process_load_requests).run_if(in_state(AppState::InGame)),
            );
    }
}

fn register_reflect_types(app: &mut App) {
    app.register_type::<Nation>()
        .register_type::<Name>()
        .register_type::<NationColor>()
        .register_type::<Capital>()
        .register_type::<Technology>()
        .register_type::<Technologies>()
        .register_type::<Good>()
        .register_type::<ResourcePool>()
        .register_type::<Stockpile>()
        .register_type::<Treasury>()
        .register_type::<ProductionChoice>()
        .register_type::<ProductionSettings>()
        .register_type::<Building>()
        .register_type::<Buildings>()
        .register_type::<BuildingKind>()
        .register_type::<Season>()
        .register_type::<Calendar>()
        .register_type::<TurnPhase>()
        .register_type::<TurnSystem>()
        .register_type::<RecruitmentCapacity>()
        .register_type::<RecruitmentQueue>()
        .register_type::<TrainingQueue>()
        .register_type::<Workforce>()
        .register_type::<Worker>()
        .register_type::<WorkerSkill>()
        .register_type::<WorkerHealth>()
        .register_type::<Civilian>()
        .register_type::<CivilianOrder>()
        .register_type::<CivilianJob>()
        .register_type::<PreviousPosition>()
        .register_type::<ActionTurn>()
        .register_type::<CivilianKind>()
        .register_type::<CivilianOrderKind>()
        .register_type::<JobType>()
        .register_type::<ProspectingKnowledge>()
        .register_type::<CivilianId>()
        .register_type::<NextCivilianId>()
        .register_type::<ProvinceId>()
        .register_type::<Province>()
        .register_type::<City>()
        .register_type::<ImprovementKind>()
        .register_type::<Depot>()
        .register_type::<Port>()
        .register_type::<RailConstruction>()
        .register_type::<Roads>()
        .register_type::<Rails>()
        .register_type::<ProvincesGenerated>()
        .register_type::<AiNation>()
        .register_type::<AiControlledCivilian>();
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
            .exclude_component::<Allocations>()
            .exclude_component::<ReservationSystem>()
            .include_resource::<Calendar>()
            .include_resource::<TurnSystem>()
            .include_resource::<Roads>()
            .include_resource::<Rails>()
            .include_resource::<ProspectingKnowledge>()
            .include_resource::<NextCivilianId>()
            .include_resource::<ProvincesGenerated>();

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
    nations: Query<
        (
            Entity,
            Option<&Name>,
            Option<&Allocations>,
            Option<&ReservationSystem>,
        ),
        With<Nation>,
    >,
) {
    let mut player_entity = None;

    for (entity, name, allocations, reservations) in nations.iter() {
        if allocations.is_none() {
            commands.entity(entity).insert(Allocations::default());
        }

        if reservations.is_none() {
            commands.entity(entity).insert(ReservationSystem::default());
        }

        // Identify player nation by name
        if name.map(|name| name.as_str() == "Player").unwrap_or(false) {
            player_entity = Some(entity);
        }
    }

    if let Some(entity) = player_entity {
        commands.queue(move |world: &mut World| {
            if let Some(nation) = PlayerNation::from_entity(world, entity) {
                if world.contains_resource::<PlayerNation>() {
                    *world.resource_mut::<PlayerNation>() = nation;
                } else {
                    world.insert_resource(nation);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use bevy::app::App;

    use bevy::ecs::message::{MessageReader, MessageWriter};
    use bevy::ecs::reflect::ReflectMapEntities;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{
        AppExtStates, AppTypeRegistry, Color, Commands, Component, Entity, MinimalPlugins, Reflect,
        ReflectComponent,
    };

    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::TilePos;

    use moonshine_save::prelude::Save;

    use crate::civilians::{Civilian, CivilianId, CivilianKind};
    use crate::economy::allocation::Allocations;
    use crate::economy::goods::Good;
    use crate::economy::nation::{Capital, Nation, NationColor, PlayerNation};
    use crate::economy::reservation::ReservationSystem;
    use crate::economy::stockpile::Stockpile;
    use crate::economy::technology::{Technologies, Technology};
    use crate::economy::transport::{Rails, Roads};
    use crate::economy::treasury::Treasury;
    use crate::economy::workforce::{RecruitmentQueue, TrainingQueue, Workforce};
    use crate::economy::{Calendar, Season};
    use crate::map::province_setup::ProvincesGenerated;
    use crate::save::{
        GameSavePlugin, LoadGameCompleted, LoadGameRequest, SaveGameCompleted, SaveGameRequest,
    };
    use crate::turn_system::{TurnPhase, TurnSystem};
    use crate::ui::menu::AppState;
    use bevy::prelude::Name;

    #[derive(Component, Reflect, Default, Clone)]
    #[reflect(Component)]
    struct SerializableComponent {
        value: i32,
    }

    fn temp_save_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rust_imperialism_{label}_{}.ron",
            rand::random::<u64>()
        ));
        path
    }

    fn init_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(AppState::InGame);
        app.add_plugins(GameSavePlugin);
        app.register_type::<SerializableComponent>();

        {
            let world = app.world_mut();
            world.insert_resource(Calendar::default());
            world.insert_resource(TurnSystem::default());
            world.insert_resource(Roads::default());
            world.insert_resource(Rails::default());
            world.insert_resource(ProvincesGenerated);
        }

        app
    }

    #[test]
    fn save_request_creates_file_and_completion_message() {
        let mut app = init_test_app();
        let path = temp_save_path("save_request");

        let request_path = path.clone();
        let _ = app.world_mut().run_system_once(
            move |mut commands: Commands, mut writer: MessageWriter<SaveGameRequest>| {
                commands.spawn((SerializableComponent { value: 42 }, Save));
                writer.write(SaveGameRequest {
                    path: Some(request_path.clone()),
                });
            },
        );

        app.update();
        app.update();

        let completions = app
            .world_mut()
            .run_system_once(|mut reader: MessageReader<SaveGameCompleted>| {
                reader.read().cloned().collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].path, path);

        let contents = fs::read_to_string(&completions[0].path).unwrap();
        assert!(contents.contains("SerializableComponent"));
        assert!(contents.contains("42"));

        fs::remove_file(completions[0].path.clone()).unwrap();
    }

    #[test]
    fn nation_entities_are_marked_for_save() {
        let mut app = init_test_app();
        let nation = app.world_mut().spawn(Nation).id();

        app.update();

        assert!(app.world().entity(nation).contains::<Save>());
    }

    #[test]
    fn civilian_component_registers_map_entities() {
        let app = init_test_app();
        let registry = app.world().resource::<AppTypeRegistry>().read();
        let registration = registry
            .get(std::any::TypeId::of::<Civilian>())
            .expect("civilian type registered");
        assert!(registration.data::<ReflectMapEntities>().is_some());
    }

    #[test]
    fn civilian_owner_is_remapped_when_loading_scene() {
        // This test verifies that Civilian's MapEntities derive is properly registered
        // and that moonshine-save will use it during load.
        // The actual remapping behavior is tested by saving_and_loading_persists_core_state.
        let registry = AppTypeRegistry::default();
        {
            let mut writer = registry.write();
            writer.register::<Civilian>();
            writer.register::<Nation>();
        }

        // Verify MapEntities is registered for Civilian
        let reader = registry.read();
        let registration = reader
            .get(std::any::TypeId::of::<Civilian>())
            .expect("Civilian type registered");
        assert!(
            registration.data::<ReflectMapEntities>().is_some(),
            "Civilian should have MapEntities reflection data"
        );
    }

    #[test]
    fn load_request_rebuilds_player_nation_runtime_state() {
        let mut app = init_test_app();
        let path = temp_save_path("load_request");

        let save_request_path = path.clone();
        let _ = app.world_mut().run_system_once(
            move |mut commands: Commands, mut writer: MessageWriter<SaveGameRequest>| {
                commands.spawn((
                    Save,
                    Nation,
                    Name::new("Player"),
                    Allocations::default(),
                    ReservationSystem::default(),
                ));
                writer.write(SaveGameRequest {
                    path: Some(save_request_path.clone()),
                });
            },
        );

        app.update();
        app.update();

        assert!(fs::metadata(&path).is_ok());

        let mut app = init_test_app();
        let load_request_path = path.clone();
        let _ =
            app.world_mut()
                .run_system_once(move |mut writer: MessageWriter<LoadGameRequest>| {
                    writer.write(LoadGameRequest {
                        path: Some(load_request_path.clone()),
                    });
                });

        app.update();
        app.update();
        app.update();

        let completions = app
            .world_mut()
            .run_system_once(|mut reader: MessageReader<LoadGameCompleted>| {
                reader.read().cloned().collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].path, path);

        let player_nation_entity = app.world().resource::<PlayerNation>().entity();
        let entity = app.world().entity(player_nation_entity);
        assert!(entity.contains::<Nation>());
        assert!(entity.contains::<Allocations>());
        assert!(entity.contains::<ReservationSystem>());

        fs::remove_file(completions[0].path.clone()).unwrap();
    }

    #[test]
    fn saving_and_loading_persists_core_state() {
        let mut app = init_test_app();
        let path = temp_save_path("core_state");

        {
            let mut calendar = app.world_mut().resource_mut::<Calendar>();
            calendar.season = Season::Autumn;
            calendar.year = 1822;
        }

        {
            let mut turn_system = app.world_mut().resource_mut::<TurnSystem>();
            turn_system.current_turn = 5;
            turn_system.phase = TurnPhase::EnemyTurn;
        }

        let nation_entity = app
            .world_mut()
            .spawn((
                Nation,
                Name::new("Rustonia"),
                NationColor(Color::srgb(0.3, 0.4, 0.8)),
                Capital(TilePos { x: 4, y: 9 }),
                Treasury::new(1_234),
                Stockpile::default(),
                Technologies::default(),
                Workforce::default(),
                RecruitmentQueue::default(),
                TrainingQueue::default(),
            ))
            .id();

        {
            let mut entity = app.world_mut().entity_mut(nation_entity);
            let mut stockpile = entity.get_mut::<Stockpile>().unwrap();
            stockpile.add(Good::Steel, 5);
            stockpile.add(Good::Grain, 12);

            let mut techs = entity.get_mut::<Technologies>().unwrap();
            techs.unlock(Technology::MountainEngineering);
        }

        app.world_mut().spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 4, y: 9 },
            owner: nation_entity,
            civilian_id: CivilianId(1),
            has_moved: false,
        });

        let save_request_path = path.clone();
        let _ =
            app.world_mut()
                .run_system_once(move |mut writer: MessageWriter<SaveGameRequest>| {
                    writer.write(SaveGameRequest {
                        path: Some(save_request_path.clone()),
                    });
                });

        app.update();
        app.update();
        assert!(fs::metadata(&path).is_ok());

        let mut app = init_test_app();
        {
            app.world_mut().resource_mut::<Calendar>().year = 1900;
            app.world_mut().resource_mut::<TurnSystem>().current_turn = 1;
        }

        let load_request_path = path.clone();
        let _ =
            app.world_mut()
                .run_system_once(move |mut writer: MessageWriter<LoadGameRequest>| {
                    writer.write(LoadGameRequest {
                        path: Some(load_request_path.clone()),
                    });
                });

        app.update();
        app.update();
        app.update();

        let calendar = app.world().resource::<Calendar>();
        assert_eq!(calendar.year, 1822);
        assert_eq!(calendar.season, Season::Autumn);

        let turn_system = app.world().resource::<TurnSystem>();
        assert_eq!(turn_system.current_turn, 5);
        assert_eq!(turn_system.phase, TurnPhase::EnemyTurn);

        {
            let world = app.world_mut();
            let mut nation_query = world.query::<(Entity, &Name, &Treasury, &Stockpile)>();
            let (nation_entity, name, treasury, stockpile) = nation_query
                .iter(world)
                .find(|(_, name, _, _)| name.as_str() == "Rustonia")
                .expect("nation restored");

            assert_eq!(name.as_str(), "Rustonia");
            assert_eq!(treasury.total(), 1_234i64);
            assert_eq!(stockpile.get(Good::Steel), 5u32);
            assert_eq!(stockpile.get(Good::Grain), 12u32);

            let mut tech_query = world.query::<&Technologies>();
            let techs = tech_query
                .get(world, nation_entity)
                .expect("technologies restored");
            assert!(techs.has(Technology::MountainEngineering));

            let mut civilian_query = world.query::<&Civilian>();
            let civilian = civilian_query
                .iter(world)
                .find(|civilian| civilian.owner == nation_entity)
                .expect("civilian restored");
            assert_eq!(civilian.kind, CivilianKind::Engineer);
        }

        fs::remove_file(path).unwrap();
    }
}
