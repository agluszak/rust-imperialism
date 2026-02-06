use bevy_ecs_tilemap::prelude::TilePos;
use criterion::{Criterion, criterion_group, criterion_main};
use rust_imperialism::map::province::ProvinceId;
use rust_imperialism::map::province_setup::calculate_adjacency;

fn setup_provinces(
    province_grid_size: u32,
    province_tile_size: u32,
) -> Vec<(ProvinceId, Vec<TilePos>)> {
    let mut provinces = Vec::new();
    let mut province_id_counter = 0;

    for px in 0..province_grid_size {
        for py in 0..province_grid_size {
            let mut tiles = Vec::new();
            let start_x = px * province_tile_size;
            let start_y = py * province_tile_size;

            for tx in 0..province_tile_size {
                for ty in 0..province_tile_size {
                    tiles.push(TilePos {
                        x: start_x + tx,
                        y: start_y + ty,
                    });
                }
            }

            provinces.push((ProvinceId(province_id_counter), tiles));
            province_id_counter += 1;
        }
    }
    provinces
}

fn bench_adjacency_calculation(c: &mut Criterion) {
    // 20x20 provinces (400 total), each 4x4 tiles (16 tiles).
    // Total tiles: 6400.
    // With O(N) optimization, this should be very fast (~4ms).

    let province_data = setup_provinces(20, 4);

    c.bench_function("calculate_adjacency", |b| {
        b.iter(|| calculate_adjacency(&province_data))
    });
}

criterion_group!(benches, bench_adjacency_calculation);
criterion_main!(benches);
