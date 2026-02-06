use bevy_ecs_tilemap::prelude::TilePos;
use criterion::{Criterion, criterion_group, criterion_main};
use rust_imperialism::ai::snapshot::calculate_suggested_depots;
use rust_imperialism::map::tiles::TerrainType;
use std::collections::{HashMap, HashSet};

fn benchmark_calculate_suggested_depots(c: &mut Criterion) {
    // Setup a large scenario
    let width = 50;
    let height = 50;
    let mut owned_tiles = HashSet::new();
    let mut tile_terrain = HashMap::new();
    let mut resource_tiles = HashSet::new();

    for x in 0..width {
        for y in 0..height {
            let pos = TilePos::new(x, y);
            owned_tiles.insert(pos);
            tile_terrain.insert(pos, TerrainType::Grass);

            // 10% chance of resource
            if (x * y) % 10 == 0 {
                resource_tiles.insert(pos);
            }
        }
    }

    let depot_positions = HashSet::new();
    let capital_pos = TilePos::new(width / 2, height / 2);

    c.bench_function("calculate_suggested_depots", |b| {
        b.iter(|| {
            calculate_suggested_depots(
                std::hint::black_box(&resource_tiles),
                std::hint::black_box(&owned_tiles),
                std::hint::black_box(&depot_positions),
                std::hint::black_box(capital_pos),
                std::hint::black_box(&tile_terrain),
            )
        })
    });
}

criterion_group!(benches, benchmark_calculate_suggested_depots);
criterion_main!(benches);
