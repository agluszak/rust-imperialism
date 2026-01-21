use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};
use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
use rust_imperialism::map::tile_pos::{HexExt, TilePosExt};
use std::collections::HashMap;

fn setup_world() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins);

    app.finish();
    app.cleanup();
    app.update();

    let map_size = TilemapSize { x: 64, y: 64 };
    let tile_storage = {
        let world = app.world_mut();
        setup_map(world, map_size)
    };

    {
        let world = app.world_mut();
        setup_provinces(world, &tile_storage, map_size);
    }

    app
}

fn bench_border_calculation(c: &mut Criterion) {
    let mut app = setup_world();
    let world = app.world_mut();

    let mut tile_storage_query = world.query::<(&TileStorage, &TilemapSize)>();
    let mut provinces_query = world.query::<&Province>();

    let (tile_storage, map_size) = tile_storage_query.iter(world).next().unwrap();
    let tile_storage = tile_storage.clone();
    let map_size = *map_size;

    let all_provinces: Vec<Province> = provinces_query.iter(world).cloned().collect();

    let mut entity_to_province = HashMap::new();
    let mut q = world.query::<(Entity, &TileProvince)>();
    for (e, tp) in q.iter(world) {
        entity_to_province.insert(e, *tp);
    }

    c.bench_function("border calculation (baseline)", |b| {
        b.iter(|| {
            let mut _lines_count = 0;

            for province in &all_provinces {
                for &tile_pos in &province.tiles {
                    if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                        if let Some(tile_prov) = entity_to_province.get(&tile_entity) {
                            let tile_hex = tile_pos.to_hex();
                            for neighbor_hex in tile_hex.all_neighbors() {
                                if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                                    if neighbor_pos.x >= map_size.x || neighbor_pos.y >= map_size.y { continue; }
                                    if let Some(neighbor_entity) = tile_storage.get(&neighbor_pos) {
                                        if let Some(neighbor_prov) = entity_to_province.get(&neighbor_entity) {
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
    let lines_count = 5000;
    let cached_lines: Vec<(Vec2, Vec2, Color)> = (0..lines_count).map(|_| (Vec2::ZERO, Vec2::ZERO, Color::WHITE)).collect();

    c.bench_function("border rendering (cached)", |b| {
        b.iter(|| {
            for (start, end, color) in &cached_lines {
                // Simulate gizmo call
                std::hint::black_box((*start, *end, *color));
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
