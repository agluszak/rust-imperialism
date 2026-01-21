use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
use rust_imperialism::map::tile_pos::{TilePosExt, HexExt};
use std::time::Instant;
use std::collections::HashMap;

#[test]
fn bench_border_optimization() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Create a world
    let world = app.world_mut();

    // Setup map
    let map_size = TilemapSize { x: 64, y: 64 };
    let tile_storage = setup_map(world, map_size);

    // Setup provinces
    setup_provinces(world, &tile_storage, map_size);

    // Extract data
    let mut tile_storage_query = world.query::<(&TileStorage, &TilemapSize)>();
    let mut tile_provinces_query = world.query::<&TileProvince>();
    let mut provinces_query = world.query::<&Province>();

    let (tile_storage, map_size) = tile_storage_query.iter(world).next().unwrap();

    // Pre-calculate expected number of lines
    let mut lines_count = 0;

    let province_owners: HashMap<ProvinceId, Option<Entity>> =
        provinces_query.iter(world).map(|p| (p.id, p.owner)).collect();

    // 1. Measure Baseline Calculation (the O(N) logic)
    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        lines_count = 0; // reset
        for province in provinces_query.iter(world) {
            for &tile_pos in &province.tiles {
                if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                    if let Ok(tile_prov) = tile_provinces_query.get(world, tile_entity) {
                        let tile_hex = tile_pos.to_hex();
                        for neighbor_hex in tile_hex.all_neighbors() {
                            if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                                if neighbor_pos.x >= map_size.x || neighbor_pos.y >= map_size.y { continue; }
                                if let Some(neighbor_entity) = tile_storage.get(&neighbor_pos) {
                                    if let Ok(neighbor_prov) = tile_provinces_query.get(world, neighbor_entity) {
                                        if tile_prov.province_id != neighbor_prov.province_id {
                                            // Mock drawing
                                            lines_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let baseline_duration = start.elapsed();
    let baseline_per_iter = baseline_duration / iterations;

    // 2. Measure Cached Performance (iterating list of lines)
    // Create a dummy cache vector
    let cached_lines: Vec<(Vec2, Vec2, Color)> = (0..lines_count).map(|_| (Vec2::ZERO, Vec2::ZERO, Color::WHITE)).collect();

    let start_cached = Instant::now();
    // Run many more iterations because it's fast
    let cached_iterations = 10000;

    for _ in 0..cached_iterations {
        for (start, end, color) in &cached_lines {
            // Mock gizmos.line_2d call
            let _ = (*start, *end, *color);
        }
    }

    let cached_duration = start_cached.elapsed();
    let cached_per_iter = cached_duration / cached_iterations;

    println!("Baseline (Calculation): {:?} per frame", baseline_per_iter);
    println!("Optimized (Cached Draw): {:?} per frame", cached_per_iter);
    println!("Speedup: {:.2}x", baseline_per_iter.as_nanos() as f64 / cached_per_iter.as_nanos() as f64);
}

fn setup_map(world: &mut World, size: TilemapSize) -> TileStorage {
    let mut storage = TileStorage::empty(size);
    let tile_size = TilemapTileSize { x: 64.0, y: 64.0 };
    let grid_size = TilemapGridSize { x: 64.0, y: 64.0 };
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    let map_entity = world.spawn((
        size,
        grid_size,
        map_type,
        tile_size,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();

    for x in 0..size.x {
        for y in 0..size.y {
            let pos = TilePos { x, y };
            let tile_entity = world.spawn(pos).id();
            storage.set(&pos, tile_entity);
        }
    }
    world.entity_mut(map_entity).insert(storage.clone());
    storage
}

fn setup_provinces(world: &mut World, storage: &TileStorage, size: TilemapSize) {
    let mut provinces = std::collections::HashMap::new();
    for x in 0..size.x {
        for y in 0..size.y {
            let prov_x = x / 8;
            let prov_y = y / 8;
            let prov_id = ProvinceId(prov_x + prov_y * 4);
            let pos = TilePos { x, y };
            if let Some(entity) = storage.get(&pos) {
                world.entity_mut(entity).insert(TileProvince { province_id: prov_id });
                provinces.entry(prov_id).or_insert_with(Vec::new).push(pos);
            }
        }
    }
    for (id, tiles) in provinces {
        world.spawn(Province {
            id,
            tiles: tiles.clone(),
            city_tile: tiles[0],
            owner: None,
        });
    }
}
