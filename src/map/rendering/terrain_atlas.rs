//! Runtime terrain atlas builder
//!
//! Loads individual terrain BMP files and combines them into a single texture atlas
//! for use with bevy_ecs_tilemap.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Resource that holds the terrain atlas once it's built
#[derive(Resource)]
pub struct TerrainAtlas {
    pub texture: Handle<Image>,
    pub ready: bool,
}

/// Marker resource to track atlas building state
#[derive(Resource, Default)]
pub struct TerrainAtlasBuilder {
    pub terrain_handles: Vec<(usize, Handle<Image>)>, // (index, handle)
    pub loaded_count: usize,
}

const TILE_SIZE: u32 = 64;
const ATLAS_TILES_WIDE: u32 = 4;
const ATLAS_TILES_HIGH: u32 = 2; // We have 8 terrain types, so 4x2 = 8 slots

/// Load all terrain tiles at startup
pub fn start_terrain_atlas_loading(mut commands: Commands, asset_server: Res<AssetServer>) {
    let terrain_files = vec![
        ("glop/pictuniv.gob_2_10000.BMP.bmp", 0), // Grass
        ("glop/pictuniv.gob_2_10001.BMP.bmp", 1), // Forest
        ("glop/pictuniv.gob_2_10002.BMP.bmp", 2), // Hills
        ("glop/pictuniv.gob_2_10003.BMP.bmp", 3), // Mountains
        ("glop/pictuniv.gob_2_10004.BMP.bmp", 4), // Swamp
        ("glop/pictuniv.gob_2_10005.BMP.bmp", 5), // Water
        ("glop/pictuniv.gob_2_10006.BMP.bmp", 6), // Desert
        ("glop/pictuniv.gob_2_10007.BMP.bmp", 7), // Farmland
    ];

    let mut builder = TerrainAtlasBuilder::default();

    for (path, index) in terrain_files {
        let handle = asset_server.load(path);
        builder.terrain_handles.push((index, handle));
    }

    commands.insert_resource(builder);
}

/// Check if all terrain tiles are loaded and build the atlas
pub fn build_terrain_atlas_when_ready(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    builder: Option<ResMut<TerrainAtlasBuilder>>,
    existing_atlas: Option<Res<TerrainAtlas>>,
) {
    // Skip if atlas already exists or builder not initialized
    if existing_atlas.is_some() || builder.is_none() {
        return;
    }

    let builder = builder.unwrap();

    // Check if all images are loaded
    let mut all_loaded = true;
    for (_, handle) in &builder.terrain_handles {
        if images.get(handle).is_none() {
            all_loaded = false;
            break;
        }
    }

    if !all_loaded {
        return;
    }

    info!("All terrain tiles loaded, building atlas...");

    // Create the atlas
    let atlas_width = TILE_SIZE * ATLAS_TILES_WIDE;
    let atlas_height = TILE_SIZE * ATLAS_TILES_HIGH;
    let mut atlas_data = vec![0u8; (atlas_width * atlas_height * 4) as usize]; // RGBA

    // Copy each tile into the atlas
    for (index, handle) in &builder.terrain_handles {
        if let Some(image) = images.get(handle) {
            let x_offset = (index % ATLAS_TILES_WIDE as usize) as u32 * TILE_SIZE;
            let y_offset = (index / ATLAS_TILES_WIDE as usize) as u32 * TILE_SIZE;

            // Access image data - clone to get owned data
            let src_data = image.clone().data;

            // Copy pixel data if data exists
            if let Some(pixel_data) = &src_data {
                for y in 0..TILE_SIZE {
                    for x in 0..TILE_SIZE {
                        let src_idx = ((y * TILE_SIZE + x) * 4) as usize;
                        let dst_idx =
                            (((y_offset + y) * atlas_width + (x_offset + x)) * 4) as usize;

                        if src_idx + 3 < pixel_data.len() && dst_idx + 3 < atlas_data.len() {
                            atlas_data[dst_idx..dst_idx + 4]
                                .copy_from_slice(&pixel_data[src_idx..src_idx + 4]);
                        }
                    }
                }

                info!(
                    "Placed terrain tile {} at ({}, {})",
                    index, x_offset, y_offset
                );
            } else {
                warn!("Tile {} has no pixel data!", index);
            }
        }
    }

    // Create the atlas image
    let atlas_image = Image::new(
        Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        atlas_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    let atlas_handle = images.add(atlas_image);

    commands.insert_resource(TerrainAtlas {
        texture: atlas_handle,
        ready: true,
    });

    info!("Terrain atlas built successfully!");
}
