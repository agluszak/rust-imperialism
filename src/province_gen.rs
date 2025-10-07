use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};
use std::collections::{HashSet, VecDeque};

use crate::province::{Province, ProvinceId, TileProvince};
use crate::tile_pos::{HexExt, TilePosExt};
use crate::tiles::TerrainType;

const MIN_PROVINCE_SIZE: usize = 15;
const MAX_PROVINCE_SIZE: usize = 20;

/// Generate provinces by flood-filling non-water tiles
pub fn generate_provinces(
    commands: &mut Commands,
    tile_storage: &TileStorage,
    tile_types: &Query<&TerrainType>,
    map_width: u32,
    map_height: u32,
) -> Vec<Entity> {
    let mut assigned_tiles: HashSet<TilePos> = HashSet::new();
    let mut provinces = Vec::new();
    let mut province_id = 0u32;

    // Collect all non-water tiles
    let mut available_tiles: Vec<TilePos> = Vec::new();
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = TilePos { x, y };
            if let Some(tile_entity) = tile_storage.get(&pos)
                && let Ok(terrain) = tile_types.get(tile_entity)
                && *terrain != TerrainType::Water
            {
                available_tiles.push(pos);
            }
        }
    }

    info!(
        "Starting province generation with {} non-water tiles",
        available_tiles.len()
    );

    // Generate provinces using flood fill
    while let Some(seed_tile) = available_tiles
        .iter()
        .find(|t| !assigned_tiles.contains(t))
        .copied()
    {
        let province_tiles = flood_fill_province(
            seed_tile,
            &assigned_tiles,
            tile_storage,
            tile_types,
            map_width,
            map_height,
        );

        if province_tiles.is_empty() {
            assigned_tiles.insert(seed_tile);
            continue;
        }

        // Mark tiles as assigned
        for tile in &province_tiles {
            assigned_tiles.insert(*tile);
        }

        // Choose city location (center-ish tile)
        let city_tile = choose_city_location(&province_tiles);

        let id = ProvinceId(province_id);
        province_id += 1;

        // Create province entity
        let province_entity = commands
            .spawn(Province::new(id, province_tiles.clone(), city_tile))
            .id();

        provinces.push(province_entity);

        // Tag all tiles with their province
        for tile_pos in &province_tiles {
            if let Some(tile_entity) = tile_storage.get(tile_pos) {
                commands
                    .entity(tile_entity)
                    .insert(TileProvince { province_id: id });
            }
        }

        info!(
            "Created province {} with {} tiles, city at ({}, {})",
            id.0,
            province_tiles.len(),
            city_tile.x,
            city_tile.y
        );
    }

    info!("Generated {} provinces total", provinces.len());
    provinces
}

/// Flood fill from a seed tile to create a province of 15-20 tiles
fn flood_fill_province(
    seed: TilePos,
    assigned: &HashSet<TilePos>,
    tile_storage: &TileStorage,
    tile_types: &Query<&TerrainType>,
    map_width: u32,
    map_height: u32,
) -> Vec<TilePos> {
    let mut province_tiles = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(seed);
    visited.insert(seed);

    while let Some(current) = queue.pop_front() {
        // Check if tile is valid
        if assigned.contains(&current) {
            continue;
        }

        if let Some(tile_entity) = tile_storage.get(&current)
            && let Ok(terrain) = tile_types.get(tile_entity)
        {
            if *terrain == TerrainType::Water {
                continue;
            }

            // Add to province
            province_tiles.push(current);

            // Stop if we've reached target size
            if province_tiles.len() >= MAX_PROVINCE_SIZE {
                break;
            }

            // Add neighbors to queue
            let hex = current.to_hex();
            for neighbor_hex in hex.all_neighbors() {
                if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                    // Check bounds
                    if neighbor_pos.x < map_width
                        && neighbor_pos.y < map_height
                        && !visited.contains(&neighbor_pos)
                    {
                        visited.insert(neighbor_pos);
                        queue.push_back(neighbor_pos);
                    }
                }
            }
        }
    }

    // Only return if we have at least MIN_PROVINCE_SIZE tiles
    if province_tiles.len() >= MIN_PROVINCE_SIZE {
        province_tiles
    } else {
        Vec::new()
    }
}

/// Choose a city location within the province (roughly central)
fn choose_city_location(tiles: &[TilePos]) -> TilePos {
    if tiles.is_empty() {
        return TilePos { x: 0, y: 0 };
    }

    // Calculate centroid
    let sum_x: u32 = tiles.iter().map(|t| t.x).sum();
    let sum_y: u32 = tiles.iter().map(|t| t.y).sum();
    let center_x = sum_x / tiles.len() as u32;
    let center_y = sum_y / tiles.len() as u32;

    // Find the tile closest to the centroid
    tiles
        .iter()
        .min_by_key(|t| {
            let dx = (t.x as i32 - center_x as i32).abs();
            let dy = (t.y as i32 - center_y as i32).abs();
            dx + dy
        })
        .copied()
        .unwrap_or(tiles[0])
}
