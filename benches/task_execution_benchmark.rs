use bevy::prelude::Entity;
use bevy_ecs_tilemap::prelude::TilePos;
use criterion::{Criterion, criterion_group, criterion_main, black_box};
use rust_imperialism::ai::planner::CivilianTask;
use rust_imperialism::ai::execute::sort_civilian_tasks_topologically;
use std::collections::HashMap;

fn create_test_data(count: u32) -> (HashMap<Entity, CivilianTask>, HashMap<TilePos, Entity>) {
    let mut tasks = HashMap::new();
    let mut positions = HashMap::new();

    for i in 0..count {
        let entity = Entity::from_bits(i as u64 + 1);
        let pos = TilePos::new(i % 100, i / 100);

        // Make a chain of dependencies: i moves to pos of i+1
        // To do this, entity i is at pos i. Target is pos i+1.
        // Entity i+1 is at pos i+1.

        // This is a simple linear chain dependency:
        // 0 -> 1 -> 2 -> ... -> N
        // 0 is at (0,0), wants to move to (1,0) where 1 is.
        // 1 is at (1,0), wants to move to (2,0) where 2 is.

        let target_pos = if i < count - 1 {
            TilePos::new((i + 1) % 100, (i + 1) / 100)
        } else {
             TilePos::new((i + 2) % 100, (i + 2) / 100)
        };

        tasks.insert(entity, CivilianTask::MoveTo { target: target_pos });
        positions.insert(pos, entity);
    }

    (tasks, positions)
}

fn bench_topological_sort(c: &mut Criterion) {
    // Large dataset to make cloning visible
    let count = 2000;
    let (tasks, positions) = create_test_data(count);

    c.bench_function("sort_civilian_tasks_topologically", |b| {
        b.iter(|| {
            sort_civilian_tasks_topologically(black_box(&tasks), black_box(&positions))
        })
    });
}

criterion_group!(benches, bench_topological_sort);
criterion_main!(benches);
