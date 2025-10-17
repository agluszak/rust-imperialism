use std::path::PathBuf;

use bevy::prelude::*;
use moonshine_save::prelude::*;

use crate::economy::allocation::Allocations;
use crate::economy::nation::{Name, NationId, PlayerNation};
use crate::economy::reservation::ReservationSystem;
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
    app.register_type::<NationId>();
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use bevy::app::App;
    use bevy::ecs::message::{MessageReader, MessageWriter};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{AppExtStates, Commands, Component, MinimalPlugins, Reflect, ReflectComponent};
    use bevy::state::app::StatesPlugin;

    use moonshine_save::prelude::Save;

    use crate::economy::allocation::Allocations;
    use crate::economy::reservation::ReservationSystem;
    use crate::economy::transport::{Rails, Roads};
    use crate::economy::Calendar;
    use crate::economy::nation::{NationId, PlayerNation};
    use crate::province_setup::ProvincesGenerated;
    use crate::save::{
        GameSavePlugin, LoadGameCompleted, LoadGameRequest, SaveGameCompleted, SaveGameRequest,
    };
    use crate::ui::menu::AppState;
    use crate::turn_system::TurnSystem;

    #[derive(Component, Reflect, Default, Clone)]
    #[reflect(Component)]
    struct SerializableComponent {
        value: i32,
    }

    fn temp_save_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("rust_imperialism_{label}_{}.ron", rand::random::<u64>()));
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
    fn load_request_rebuilds_player_nation_runtime_state() {
        let mut app = init_test_app();
        let path = temp_save_path("load_request");

        let save_request_path = path.clone();
        let _ = app.world_mut().run_system_once(
            move |mut commands: Commands, mut writer: MessageWriter<SaveGameRequest>| {
                commands.spawn((
                    Save,
                    NationId(1),
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
        let _ = app.world_mut().run_system_once(move |mut writer: MessageWriter<LoadGameRequest>| {
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

        let player_nation_entity = app.world().resource::<PlayerNation>().0;
        let entity = app.world().entity(player_nation_entity);
        assert_eq!(entity.get::<NationId>().unwrap().0, 1);
        assert!(entity.contains::<Allocations>());
        assert!(entity.contains::<ReservationSystem>());

        fs::remove_file(completions[0].path.clone()).unwrap();
    }
}
