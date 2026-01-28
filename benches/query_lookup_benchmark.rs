use bevy::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};

#[derive(Component)]
struct Civilian;

fn bench_entity_lookup(c: &mut Criterion) {
    let mut world = World::new();
    let mut entities = Vec::new();

    // Spawn 10,000 entities
    for _ in 0..10_000 {
        entities.push(world.spawn(Civilian).id());
    }

    // Pick a target in the middle
    let target = entities[5000];

    // Create a query state
    let mut query = world.query::<(Entity, &Civilian)>();

    let mut group = c.benchmark_group("entity_lookup");

    group.bench_function("iter_find", |b| {
        b.iter(|| {
            let found = query.iter(&world).find(|(e, _)| *e == target);
            std::hint::black_box(found);
        })
    });

    group.bench_function("get", |b| {
        b.iter(|| {
            let found = query.get(&world, target);
            let _ = std::hint::black_box(found);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_entity_lookup);
criterion_main!(benches);
