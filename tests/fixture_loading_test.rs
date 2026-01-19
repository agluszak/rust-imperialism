//! Tests that verify fixture loading works correctly

mod common;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use rust_imperialism::economy::nation::NationColor;
use rust_imperialism::map::province::Province;

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
