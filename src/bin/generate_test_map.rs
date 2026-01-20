//! One-time generation of pruned test maps for integration tests.
//! Run with: cargo run --bin generate_test_map

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::*;
use moonshine_save::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rust_imperialism::ai::AiControlledCivilian;
use rust_imperialism::ai::AiNation;
use rust_imperialism::civilians::Civilian;
use rust_imperialism::constants::TERRAIN_SEED;
use rust_imperialism::economy::nation::NationColor;
use rust_imperialism::economy::transport::Rails;
use rust_imperialism::map::TerrainType;
use rust_imperialism::map::prospecting::PotentialMineral;
use rust_imperialism::map::province::Province;
use rust_imperialism::map::province::TileProvince;
use rust_imperialism::map::province_setup::TestMapConfig;
use rust_imperialism::map::terrain_gen::TerrainGenerator;
use rust_imperialism::resources::{ResourceType, TileResource};
use rust_imperialism::save::GameSavePlugin;
use rust_imperialism::turn_system::TurnPhase;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;

fn main() {
    println!("Generating pruned test map...");

    let mut app = App::new();

    // Minimal plugins for headless generation
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Initialize game states
    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    // Add save plugin (handles reflection registration)
    app.add_plugins(GameSavePlugin);

    // Add resources normally provided by other plugins
    app.init_resource::<rust_imperialism::civilians::NextCivilianId>();
    app.insert_resource(Rails::default());

    // Insert test config to trigger pruning
    app.insert_resource(TestMapConfig);

    // Track generation state
    app.init_resource::<GenerationState>();

    // Add observer to handle save completion
    app.add_observer(on_save_complete);

    // Add systems for map generation and saving
    app.add_systems(
        Update,
        (
            setup_mock_tilemap,
            rust_imperialism::map::province_setup::generate_provinces_system,
            rust_imperialism::map::province_setup::assign_provinces_to_countries,
            ensure_red_nation_for_target_tile,
            rust_imperialism::map::province_setup::prune_to_test_map
                .run_if(resource_exists::<TargetRedReady>),
            mark_tiles_for_save,
            trigger_save,
        )
            .chain()
            .run_if(in_state(AppState::InGame)),
    );

    // Run until save completes
    loop {
        app.update();
        if app.world().resource::<GenerationState>().done {
            break;
        }
    }

    println!("Test map saved successfully!");
}

#[derive(Resource, Default)]
struct GenerationState {
    tilemap_ready: bool,
    pruning_done: bool,
    tiles_marked: bool,
    done: bool,
}

#[derive(Resource)]
struct TargetRedReady;

fn setup_mock_tilemap(
    mut commands: Commands,
    tilemap_query: Query<&TileStorage>,
    mut state: ResMut<GenerationState>,
) {
    if state.tilemap_ready || !tilemap_query.is_empty() {
        state.tilemap_ready = true;
        return;
    }

    println!("Creating tilemap...");

    let map_size = TilemapSize { x: 32, y: 32 };
    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);
    let terrain_gen = TerrainGenerator::new(TERRAIN_SEED);
    let mut rng = StdRng::seed_from_u64(TERRAIN_SEED as u64);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let terrain_type = terrain_gen.generate_terrain(x, y, map_size.x, map_size.y);
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(terrain_type.get_texture_index()),
                        ..default()
                    },
                    terrain_type,
                ))
                .id();

            match terrain_type {
                TerrainType::Farmland => {
                    let roll = rng.random::<f32>();
                    let resource = if roll < 0.7 {
                        ResourceType::Grain
                    } else if roll < 0.9 {
                        ResourceType::Cotton
                    } else {
                        ResourceType::Fruit
                    };
                    commands
                        .entity(tile_entity)
                        .insert(TileResource::visible(resource));
                }
                TerrainType::Grass => {
                    if rng.random::<f32>() < 0.4 {
                        let resource = if rng.random::<bool>() {
                            ResourceType::Wool
                        } else {
                            ResourceType::Livestock
                        };
                        commands
                            .entity(tile_entity)
                            .insert(TileResource::visible(resource));
                    }
                }
                TerrainType::Forest => {
                    commands
                        .entity(tile_entity)
                        .insert(TileResource::visible(ResourceType::Timber));
                }
                TerrainType::Mountain => {
                    let has_mineral = rng.random::<f32>() < 0.6;
                    let mineral_type = if has_mineral {
                        let roll = rng.random::<f32>();
                        if roll < 0.4 {
                            Some(ResourceType::Coal)
                        } else if roll < 0.7 {
                            Some(ResourceType::Iron)
                        } else if roll < 0.9 {
                            Some(ResourceType::Gold)
                        } else {
                            Some(ResourceType::Gems)
                        }
                    } else {
                        None
                    };
                    commands
                        .entity(tile_entity)
                        .insert(PotentialMineral::new(mineral_type));
                }
                TerrainType::Hills => {
                    let has_mineral = rng.random::<f32>() < 0.4;
                    let mineral_type = if has_mineral {
                        let roll = rng.random::<f32>();
                        if roll < 0.6 {
                            Some(ResourceType::Coal)
                        } else {
                            Some(ResourceType::Iron)
                        }
                    } else {
                        None
                    };
                    commands
                        .entity(tile_entity)
                        .insert(PotentialMineral::new(mineral_type));
                }
                TerrainType::Desert => {
                    let has_oil = rng.random::<f32>() < 0.15;
                    let mineral_type = if has_oil {
                        Some(ResourceType::Oil)
                    } else {
                        None
                    };
                    commands
                        .entity(tile_entity)
                        .insert(PotentialMineral::new(mineral_type));
                }
                TerrainType::Water | TerrainType::Swamp => {}
            }
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert((
        TilemapGridSize { x: 16.0, y: 16.0 },
        TilemapType::Hexagon(HexCoordSystem::Row),
        map_size,
        tile_storage,
        TilemapTileSize { x: 16.0, y: 16.0 },
    ));

    state.tilemap_ready = true;
}

fn ensure_red_nation_for_target_tile(
    mut commands: Commands,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut nations: Query<(Entity, &mut NationColor, Option<&AiNation>)>,
    civilians: Query<(Entity, &Civilian)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    let Some(tile_storage) = tile_storage_query.iter().next() else {
        return;
    };

    let target = TilePos { x: 25, y: 7 };
    let Some(tile_entity) = tile_storage.get(&target) else {
        return;
    };
    let Ok(tile_province) = tile_provinces.get(tile_entity) else {
        return;
    };
    let Some(target_owner) = provinces
        .iter()
        .find(|province| province.id == tile_province.province_id)
        .and_then(|province| province.owner)
    else {
        return;
    };

    let red_color = Color::srgb(0.8, 0.2, 0.2);
    let mut red_entity = None;
    let mut target_owner_color = None;

    for (entity, color, _) in nations.iter() {
        let linear = color.0.to_linear();
        let expected = red_color.to_linear();
        if (linear.red - expected.red).abs() < 0.01
            && (linear.green - expected.green).abs() < 0.01
            && (linear.blue - expected.blue).abs() < 0.01
        {
            red_entity = Some(entity);
        }
        if entity == target_owner {
            target_owner_color = Some(color.0);
        }
    }

    let Some(target_owner_color) = target_owner_color else {
        return;
    };

    if red_entity != Some(target_owner) {
        if let Some(red_entity) = red_entity
            && let Ok((_, mut color, _)) = nations.get_mut(red_entity)
        {
            color.0 = target_owner_color;
        }

        if let Ok((_, mut color, ai_marker)) = nations.get_mut(target_owner) {
            color.0 = red_color;
            if ai_marker.is_none() {
                commands.entity(target_owner).insert(AiNation);
            }
        }

        for (entity, civilian) in civilians.iter() {
            if civilian.owner == target_owner {
                commands.entity(entity).insert(AiControlledCivilian);
            }
        }
    } else if let Ok((_, _, ai_marker)) = nations.get_mut(target_owner)
        && ai_marker.is_none()
    {
        commands.entity(target_owner).insert(AiNation);
    }

    commands.insert_resource(TargetRedReady);
    *done = true;
}

fn mark_tiles_for_save(
    mut commands: Commands,
    test_config: Option<Res<TestMapConfig>>,
    tiles: Query<Entity, With<TilePos>>,
    mut state: ResMut<GenerationState>,
) {
    // Wait for pruning to complete (TestMapConfig gets removed)
    if test_config.is_some() || state.tiles_marked {
        return;
    }

    if !state.pruning_done {
        println!("Pruning complete, marking tiles for save...");
        state.pruning_done = true;
    }

    // Mark remaining tiles with Save component
    for entity in tiles.iter() {
        commands.entity(entity).insert(Save);
    }

    state.tiles_marked = true;
}

fn trigger_save(mut commands: Commands, state: Res<GenerationState>, mut triggered: Local<bool>) {
    if !state.tiles_marked || *triggered {
        return;
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pruned_red_nation.ron");

    println!("Saving to: {:?}", path);

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create fixtures directory");
    }

    // Use moonshine-save directly
    let event = SaveWorld::default_into_file(path)
        .exclude_component::<rust_imperialism::economy::allocation::Allocations>()
        .exclude_component::<rust_imperialism::economy::reservation::ReservationSystem>()
        .include_resource::<rust_imperialism::economy::Calendar>()
        .include_resource::<rust_imperialism::turn_system::TurnCounter>()
        .include_resource::<rust_imperialism::economy::transport::Rails>()
        .include_resource::<rust_imperialism::civilians::ProspectingKnowledge>()
        .include_resource::<rust_imperialism::civilians::NextCivilianId>()
        .include_resource::<rust_imperialism::map::province_setup::ProvincesGenerated>();

    commands.trigger_save(event);
    *triggered = true;
}

fn on_save_complete(_trigger: On<Saved>, mut state: ResMut<GenerationState>) {
    state.done = true;
}
