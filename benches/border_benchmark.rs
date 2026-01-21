use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};
use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
use rust_imperialism::map::tile_pos::{HexExt, TilePosExt};
use std::collections::HashMap;

fn setup_world() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::gizmos::GizmoPlugin);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default_nearest());

    app.finish();
    app.cleanup();
    app.update(); // Initialize plugins

    let map_size = TilemapSize { x: 64, y: 64 };
    let tile_storage = {
        let world = app.world_mut();
        setup_map(world, map_size)
    };

    {
        let world = app.world_mut();
        setup_provinces(world, &tile_storage, map_size);
    }

    // We need an entity to return to keep things alive? No, just the app.
    // But we need to query the world.
    (app, Entity::PLACEHOLDER)
}

fn bench_border_calculation(c: &mut Criterion) {
    let (mut app, _) = setup_world();
    let world = app.world_mut();

    let mut tile_storage_query = world.query::<(&TileStorage, &TilemapSize)>();
    let mut tile_provinces_query = world.query::<&TileProvince>();
    let mut provinces_query = world.query::<&Province>();

    let (tile_storage, map_size) = tile_storage_query.iter(world).next().unwrap();
    let tile_storage = tile_storage.clone();
    let map_size = *map_size;

    // Pre-calculate province owners map to match the system
    let province_owners: HashMap<ProvinceId, Option<Entity>> =
        provinces_query.iter(world).map(|p| (p.id, p.owner)).collect();

    c.bench_function("border calculation (baseline)", |b| {
        b.iter(|| {
            let mut _lines_count = 0;
            // Re-acquire query access inside the loop? No, queries on world are fine if world is mutable?
            // World access inside closure is tricky if we don't have it.
            // We can't easily use `world` inside the closure if it's owned by `app` outside.
            // But we can extract the data into vectors/maps to simulate the calculation cost purely.
            // Or we can just use the world if we accept the overhead of `world.query`.

            // To properly benchmark the logic, we should probably extract the data needed *before* the bench loop
            // to just benchmark the algorithm, OR include the query iteration overhead which is part of the system.
            // Let's try to include query iteration.

            // Problem: `world` cannot be borrowed mutably inside `iter` if it's also borrowed outside.
            // Actually `world` is `&mut World`.

            // Let's assume we can just pass `&World` if we don't mutate.
            // `query.iter` takes `&World`.

            let world = &app.world(); // Immutable borrow

            // We need to re-create queries or use `world.query` inside?
            // `world.query` creates a QueryState which needs mutable access to world to update archetypes?
            // No, `world.query` is one-shot.
            // `app.world().query()` creates `QueryState`.
            // Better to create QueryState once.

            // But `QueryState::get` takes `&World`.
            // So:

            // Wait, `QueryState` requires mutable access to update archetypes usually.
            // But `iter` on `QueryState` takes `&World` (and `&mut QueryState`).

            // So we need `&mut QueryState` inside the loop.

            // We can construct QueryStates outside.

            // Let's try to simulate the data structures to avoid fighting the borrow checker with Bevy World in a closure.
            // We have `tile_storage` (map of Pos -> Entity).
            // We need `tile_provinces` (Entity -> ProvinceId).
            // We need `provinces` (list of Province components).

            // Let's extract everything to pure Rust structs.

            // ... actually, the system uses queries. If we optimize, we are optimizing the logic *using* queries.
            // If caching is 800x faster, the overhead of queries is negligible for the "Optimized" case (since it iterates cache),
            // but significant for the "Baseline" case (fetching components).

            // However, the "Baseline" is the O(N) calculation which dominates.
            // So simulating with pure structs is a fair proxy for the algorithmic complexity.

            // Wait, I can just use `world` if I do it right.

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
                                                _lines_count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    });
}

fn bench_cached_draw(c: &mut Criterion) {
    // Generate dummy cache
    let lines_count = 5000; // Approx count for 64x64 map
    let cached_lines: Vec<(Vec2, Vec2, Color)> = (0..lines_count).map(|_| (Vec2::ZERO, Vec2::ZERO, Color::WHITE)).collect();

    c.bench_function("border rendering (cached)", |b| {
        b.iter(|| {
            for (start, end, color) in &cached_lines {
                let _ = (*start, *end, *color);
            }
        })
    });
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

criterion_group!(benches, bench_border_calculation, bench_cached_draw);
criterion_main!(benches);
