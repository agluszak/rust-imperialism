use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::constants::{MAP_SIZE, TERRAIN_SEED, TILE_SIZE};
use crate::input::handle_tile_click;
use crate::resources::{ResourceType, TileResource};
use crate::ui::components::MapTilemap;
use crate::ui::menu::AppState;

// Map-related modules
pub mod province;
pub mod province_gen;
pub mod province_setup;
pub mod rendering;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;

// Re-exports for convenience
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
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// System that creates the tilemap once the terrain atlas is ready
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

            // Add resources to farmland tiles
            if terrain_type == tiles::TerrainType::Farmland {
                tile_entity_commands.insert(TileResource::visible(ResourceType::Grain));
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

    info!("Tilemap created successfully!");
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
