use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::constants::{MAP_SIZE, TERRAIN_SEED, TILE_SIZE};
use crate::input::handle_tile_click;
use crate::resources::{ResourceType, TileResource};
use crate::ui::components::MapTilemap;
use crate::ui::menu::AppState;

// Map-related modules
pub mod prospecting;
pub mod province;
pub mod province_gen;
pub mod province_setup;
pub mod rendering;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;

// Re-exports for convenience
pub use prospecting::*;
pub use province::*;
pub use province_gen::*;
pub use province_setup::*;
pub use terrain_gen::*;
pub use tile_pos::*;
pub use tiles::*;

/// Marker resource to track if tilemap has been created
#[derive(Resource)]
pub struct TilemapCreated;

/// Plugin that handles map initialization and tilemap creation
pub struct MapSetupPlugin;

impl Plugin for MapSetupPlugin {
    fn build(&self, app: &mut App) {
        // Terrain atlas loading
        app.add_systems(
            Startup,
            rendering::terrain_atlas::start_terrain_atlas_loading,
        )
        .add_systems(
            Update,
            rendering::terrain_atlas::build_terrain_atlas_when_ready,
        );

        // Tilemap creation (waits for atlas to be ready)
        app.add_systems(Update, create_tilemap.run_if(in_state(AppState::InGame)));

        // Province generation (runs after tilemap is created)
        app.add_systems(
            Update,
            (
                province_setup::generate_provinces_system,
                province_setup::assign_provinces_to_countries
                    .after(province_setup::generate_provinces_system),
                province_setup::prune_to_test_map
                    .after(province_setup::assign_provinces_to_countries),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn create_tilemap(
    mut commands: Commands,
    terrain_atlas: Option<Res<rendering::TerrainAtlas>>,
    tilemap_created: Option<Res<TilemapCreated>>,
) {
    // Skip if tilemap already created
    if tilemap_created.is_some() {
        return;
    }

    // Wait for the terrain atlas to be built
    let Some(atlas) = terrain_atlas else {
        return;
    };

    if !atlas.ready {
        return;
    }

    info!("Terrain atlas ready, creating tilemap...");

    let map_size = TilemapSize {
        x: MAP_SIZE,
        y: MAP_SIZE,
    };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    // Create terrain generator with a fixed seed for consistent worlds
    let terrain_gen = TerrainGenerator::new(TERRAIN_SEED);

    // Use deterministic RNG for resource placement (based on TERRAIN_SEED)
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(TERRAIN_SEED as u64);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };

            // Generate terrain using noise functions
            let terrain_type = terrain_gen.generate_terrain(x, y, map_size.x, map_size.y);
            let texture_index = terrain_type.get_texture_index();

            let mut tile_entity_commands = commands.spawn((
                TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(texture_index),
                    ..default()
                },
                terrain_type, // Add the terrain type component
            ));

            // Assign resources based on terrain type
            match terrain_type {
                tiles::TerrainType::Farmland => {
                    // Farmland: Grain (70%), Cotton (20%), or Fruit (10%)
                    let roll = rng.random::<f32>();
                    let resource = if roll < 0.7 {
                        ResourceType::Grain
                    } else if roll < 0.9 {
                        ResourceType::Cotton
                    } else {
                        ResourceType::Fruit
                    };
                    tile_entity_commands.insert(TileResource::visible(resource));
                }
                tiles::TerrainType::Grass => {
                    // Grassland: 40% chance of Wool or Livestock
                    if rng.random::<f32>() < 0.4 {
                        let resource = if rng.random::<bool>() {
                            ResourceType::Wool
                        } else {
                            ResourceType::Livestock
                        };
                        tile_entity_commands.insert(TileResource::visible(resource));
                    }
                }
                tiles::TerrainType::Forest => {
                    // Forest: Always has Timber
                    tile_entity_commands.insert(TileResource::visible(ResourceType::Timber));
                }
                tiles::TerrainType::Mountain => {
                    // Mountains: All can be prospected
                    // 60% chance of actual mineral: Coal, Iron, Gold, or Gems
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
                    tile_entity_commands.insert(PotentialMineral::new(mineral_type));
                }
                tiles::TerrainType::Hills => {
                    // Hills: All can be prospected
                    // 40% chance of actual mineral: Coal or Iron only (no gold/gems!)
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
                    tile_entity_commands.insert(PotentialMineral::new(mineral_type));
                }
                tiles::TerrainType::Desert => {
                    // Desert: All can be prospected for oil
                    // 15% chance of Oil
                    let has_oil = rng.random::<f32>() < 0.15;
                    let mineral_type = if has_oil {
                        Some(ResourceType::Oil)
                    } else {
                        None
                    };
                    tile_entity_commands.insert(PotentialMineral::new(mineral_type));
                }
                tiles::TerrainType::Water | tiles::TerrainType::Swamp => {
                    // Water and Swamp: No resources
                }
            }

            let tile_entity = tile_entity_commands
                .observe(handle_tile_click)
                .observe(handle_tile_hover)
                .observe(handle_tile_out)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    // Log some sample tile positions to debug rendering
    let sample_tiles = vec![
        TilePos { x: 0, y: 0 },
        TilePos { x: 15, y: 15 },
        TilePos { x: 31, y: 31 },
    ];
    for tile in sample_tiles {
        let world_pos = tile.to_world_pos();
        info!(
            "Tile ({}, {}) world pos: ({:.1}, {:.1})",
            tile.x, tile.y, world_pos.x, world_pos.y
        );
    }

    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = crate::constants::get_hex_grid_size();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    commands.entity(tilemap_entity).insert((
        TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(atlas.texture.clone()),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..Default::default()
        },
        MapTilemap, // Marker to control visibility
    ));

    // Mark tilemap as created
    commands.insert_resource(TilemapCreated);

    info!("Tilemap created successfully with resources!");
}

/// Track when mouse enters a tile
fn handle_tile_hover(
    trigger: On<Pointer<Over>>,
    tile_positions: Query<&TilePos>,
    mut hovered_tile: ResMut<rendering::HoveredTile>,
) {
    if let Ok(tile_pos) = tile_positions.get(trigger.entity) {
        hovered_tile.0 = Some(*tile_pos);
    }
}

/// Track when mouse leaves a tile
fn handle_tile_out(_trigger: On<Pointer<Out>>, mut hovered_tile: ResMut<rendering::HoveredTile>) {
    hovered_tile.0 = None;
}
