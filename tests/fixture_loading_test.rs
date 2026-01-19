//! Tests that verify fixture loading works correctly

mod common;

use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use rust_imperialism::economy::nation::{Capital, NationColor};
use rust_imperialism::economy::transport::{build_rail_graph, Rails};
use rust_imperialism::map::province::Province;
use rust_imperialism::turn_system::TurnPhase;

#[test]
fn test_load_pruned_red_nation_fixture() {
    let mut app = common::create_fixture_test_app();

    let loaded = common::load_fixture(&mut app, common::PRUNED_RED_NATION_MAP);
    assert!(loaded, "Failed to load fixture");

    let world = app.world_mut();

    // Verify only Red nation exists
    let red_color = Color::srgb(0.8, 0.2, 0.2);
    let mut nations_query = world.query::<(Entity, &NationColor)>();
    let nations: Vec<_> = nations_query.iter(world).collect();

    assert_eq!(nations.len(), 1, "Should have exactly one nation");

    let (_red_nation, color) = nations[0];
    let linear = color.0.to_linear();
    let expected = red_color.to_linear();
    assert!(
        (linear.red - expected.red).abs() < 0.01
            && (linear.green - expected.green).abs() < 0.01
            && (linear.blue - expected.blue).abs() < 0.01,
        "Nation should be Red"
    );

    // Verify provinces were loaded
    // Note: Entity remapping for Province.owner is a known limitation with moonshine-save.
    // The provinces are loaded but their owner references may not be remapped correctly.
    // For now, we just verify provinces exist with owners set.
    let mut provinces_query = world.query::<&Province>();
    let mut province_count = 0;
    for province in provinces_query.iter(world) {
        assert!(province.owner.is_some(), "Province should have an owner");
        province_count += 1;
    }
    assert!(province_count > 0, "Should have provinces");

    // Verify tile positions were loaded (tilemap types registered)
    let tile_count = world.query::<&TilePos>().iter(world).count();
    assert!(tile_count > 0, "Should have tile positions");
}

#[test]
fn test_red_nation_has_connected_rail_after_20_turns() {
    let mut app = common::create_fixture_simulation_app();

    let loaded = common::load_fixture(&mut app, common::PRUNED_RED_NATION_MAP);
    assert!(loaded, "Failed to load fixture");

    common::rebuild_tile_storage(&mut app);

    for _ in 0..20 {
        common::transition_to_phase(&mut app, TurnPhase::Processing);
        common::transition_to_phase(&mut app, TurnPhase::EnemyTurn);
        common::transition_to_phase(&mut app, TurnPhase::PlayerTurn);
    }

    let world = app.world_mut();

    let red_color = Color::srgb(0.8, 0.2, 0.2);
    let mut nations_query = world.query::<(Entity, &NationColor, &Capital)>();
    let red_nation = nations_query
        .iter(world)
        .find(|(_, color, _)| {
            let linear = color.0.to_linear();
            let expected = red_color.to_linear();
            (linear.red - expected.red).abs() < 0.01
                && (linear.green - expected.green).abs() < 0.01
                && (linear.blue - expected.blue).abs() < 0.01
        });

    let (_, _, capital) = red_nation.expect("Red nation with capital should exist");

    let rails = world.resource::<Rails>();
    let target = TilePos { x: 25, y: 7 };

    let has_target_edge = rails
        .0
        .iter()
        .any(|(a, b)| *a == target || *b == target);
    assert!(
        has_target_edge,
        "Expected rail edge to include target tile ({}, {})",
        target.x,
        target.y
    );

    let connected_tiles = connected_tiles_from(capital.0, rails);
    assert!(
        connected_tiles.contains(&target),
        "Expected rail at ({}, {}) to be connected to the red capital",
        target.x,
        target.y
    );
}

fn connected_tiles_from(start: TilePos, rails: &Rails) -> HashSet<TilePos> {
    let graph = build_rail_graph(rails);
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();

    reachable.insert(start);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        let Some(neighbors) = graph.get(&current) else {
            continue;
        };

        for &neighbor in neighbors {
            if reachable.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }

    reachable
}
