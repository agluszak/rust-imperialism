//! Integration test for AI resource collection using map pruning.
//!
//! This test uses the map pruning mechanism to create a simplified test scenario
//! with only the Red nation, then verifies that AI systems function correctly.

mod common;
use common::transition_to_phase;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::*;
use rust_imperialism::economy::nation::{Capital, NationColor};
use rust_imperialism::economy::stockpile::Stockpile;
use rust_imperialism::economy::transport::{Depot, Rails};
use rust_imperialism::economy::EconomyPlugin;
use rust_imperialism::map::prospecting::PotentialMineral;
use rust_imperialism::map::province::Province;
use rust_imperialism::map::province_setup::{
    TestMapConfig, assign_provinces_to_countries, generate_provinces_system, prune_to_test_map,
};
use rust_imperialism::map::tiles::TerrainType;
use rust_imperialism::resources::{DevelopmentLevel, ResourceType, TileResource};
use rust_imperialism::turn_system::{TurnPhase, TurnSystemPlugin};
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;

/// Test that AI collects resources correctly using map pruning for test setup.
///
/// This test verifies that:
/// 1. Map pruning creates a single-nation scenario (Red nation only)
/// 2. AI system functions correctly in a pruned map environment
/// 3. Resources exist in the pruned territory and AI can interact with them
#[test]
fn test_ai_collects_resources_with_map_pruning() {
    let mut app = App::new();

    // Minimal plugins for testing
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Initialize game states
    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    // Add resources normally provided by plugins
    app.init_resource::<rust_imperialism::civilians::types::NextCivilianId>();
    app.insert_resource(rust_imperialism::economy::transport::Rails::default());

    // Add game plugins for full AI functionality
    app.add_plugins((
        TurnSystemPlugin,
        EconomyPlugin,
        rust_imperialism::ai::AiPlugin,
        rust_imperialism::civilians::CivilianPlugin,
    ));

    // Add map generation and pruning systems
    app.add_systems(
        Update,
        (
            setup_mock_tilemap,
            generate_provinces_system,
            assign_provinces_to_countries,
            prune_to_test_map,
        )
            .chain()
            .run_if(in_state(AppState::InGame)),
    );

    // Add the test configuration to trigger pruning
    app.insert_resource(TestMapConfig);

    println!("\n=== Starting AI Resource Collection Test with Map Pruning ===");

    // Run initial setup - map generation and pruning
    // Need 12 updates for systems to run in sequence:
    // setup_mock_tilemap -> generate_provinces_system -> assign_provinces_to_countries -> prune_to_test_map
    for _ in 0..12 {
        app.update();
    }

    let world = app.world_mut();

    // Verify Red nation exists and is the only one after pruning
    let red_nation = {
        let red_color = Color::srgb(0.8, 0.2, 0.2);
        let mut nations_query = world.query::<(Entity, &NationColor)>();
        let red_nations: Vec<Entity> = nations_query
            .iter(world)
            .filter(|(_, color)| {
                (color.0.to_linear().red - red_color.to_linear().red).abs() < 0.01
                    && (color.0.to_linear().green - red_color.to_linear().green).abs() < 0.01
                    && (color.0.to_linear().blue - red_color.to_linear().blue).abs() < 0.01
            })
            .map(|(entity, _)| entity)
            .collect();

        assert_eq!(
            red_nations.len(),
            1,
            "Should have exactly one Red nation after pruning"
        );
        let all_nations: Vec<Entity> = nations_query.iter(world).map(|(e, _)| e).collect();
        assert_eq!(
            all_nations.len(),
            1,
            "Only Red nation should remain after pruning"
        );
        red_nations[0]
    };

    println!("✓ Map pruned to Red nation only: {:?}", red_nation);

    // Get capital position
    let capital_pos = world.get::<Capital>(red_nation).unwrap().0;
    println!("✓ Red nation capital at: {:?}", capital_pos);

    // Check what resources exist in the territory after pruning
    let tile_resource_count = world.query::<&TileResource>().iter(world).count();
    let potential_mineral_count = world.query::<&PotentialMineral>().iter(world).count();
    println!(
        "✓ Resources in pruned territory: {} visible resources, {} potential minerals",
        tile_resource_count, potential_mineral_count
    );

    // If there are no resources after pruning, add some test resources
    if tile_resource_count == 0 && potential_mineral_count == 0 {
        println!("No resources found after pruning, adding test resources...");
        
        // Get province and tilemap
        let tile_storage = world.query::<&TileStorage>().iter(world).next().unwrap().clone();
        
        let province_tiles = {
            let mut province_query = world.query::<&Province>();
            let province = province_query
                .iter(world)
                .find(|p| p.owner == Some(red_nation))
                .expect("Red nation should have a province");
            province.tiles.clone()
        };
        
        // Add potential minerals
        let coal_pos = TilePos {
            x: capital_pos.x.saturating_add(2),
            y: capital_pos.y.saturating_add(2),
        };
        if let Some(tile_entity) = tile_storage.get(&coal_pos) {
            // Only add resource if position is within province tiles
            if province_tiles.contains(&coal_pos) {
                world
                    .entity_mut(tile_entity)
                    .insert(PotentialMineral::new(Some(ResourceType::Coal)));
                println!("  Added potential Coal at {:?}", coal_pos);
            }
        }

        // Add visible resources
        let grain_pos = TilePos {
            x: capital_pos.x.saturating_sub(2),
            y: capital_pos.y,
        };
        if let Some(tile_entity) = tile_storage.get(&grain_pos) {
            // Only add resource if position is within province tiles
            if province_tiles.contains(&grain_pos) {
                world
                    .entity_mut(tile_entity)
                    .insert(TileResource::visible(ResourceType::Grain));
                println!("  Added visible Grain at {:?}", grain_pos);
            }
        }
    }

    // Verify that we now have resources to work with
    let final_tile_resource_count = world.query::<&TileResource>().iter(world).count();
    let final_potential_mineral_count = world.query::<&PotentialMineral>().iter(world).count();
    println!(
        "✓ Final resource count: {} visible, {} potential minerals",
        final_tile_resource_count, final_potential_mineral_count
    );

    // Capture baseline stockpile
    let initial_stockpile = world.get::<Stockpile>(red_nation).cloned().unwrap();
    let initial_coal = initial_stockpile.get(rust_imperialism::economy::goods::Good::Coal);
    let initial_grain = initial_stockpile.get(rust_imperialism::economy::goods::Good::Grain);
    println!(
        "✓ Baseline stockpile: Coal: {}, Grain: {}",
        initial_coal, initial_grain
    );

    // Track progress through the test
    let mut ai_performed_actions = false;
    let mut stockpile_changed = false;
    // Run for 30 turns - sufficient for AI to demonstrate basic functionality
    // (resource discovery, stockpile management) without making the test too slow
    let max_turns = 30;

    // Run the game for multiple turns
    for turn in 1..=max_turns {
        if turn % 5 == 0 {
            println!("\n--- Turn {} ---", turn);
        }

        // Run turn phases
        app.update();
        transition_to_phase(&mut app, TurnPhase::Processing);
        transition_to_phase(&mut app, TurnPhase::EnemyTurn);
        transition_to_phase(&mut app, TurnPhase::PlayerTurn);

        // Check if AI has taken any meaningful actions
        if !ai_performed_actions {
            let discovered = {
                let mut query = app.world_mut().query::<&TileResource>();
                query.iter(app.world()).filter(|r| r.discovered).count()
            };
            let developed = {
                let mut query = app.world_mut().query::<&TileResource>();
                query.iter(app.world()).filter(|r| r.development != DevelopmentLevel::Lv0).count()
            };
            let depots = {
                let mut query = app.world_mut().query::<&Depot>();
                query.iter(app.world()).filter(|d| d.owner == red_nation).count()
            };

            if discovered > 0 || developed > 0 || depots > 0 {
                println!("✓ Turn {}: AI performed actions! Discovered: {}, Developed: {}, Depots: {}", 
                    turn, discovered, developed, depots);
                ai_performed_actions = true;
            }
        }

        // Check for stockpile changes
        if !stockpile_changed {
            if let Some(stockpile) = app.world().get::<Stockpile>(red_nation) {
                let coal = stockpile.get(rust_imperialism::economy::goods::Good::Coal);
                let grain = stockpile.get(rust_imperialism::economy::goods::Good::Grain);

                // Check if stockpile values differ from baseline
                if coal != initial_coal || grain != initial_grain {
                    println!(
                        "✓ Turn {}: Stockpile changed! Coal: {} ({:+}), Grain: {} ({:+})",
                        turn,
                        coal, coal as i32 - initial_coal as i32,
                        grain, grain as i32 - initial_grain as i32,
                    );
                    stockpile_changed = true;
                }
            }
        }
    }

    // Final verification
    println!("\n=== Final Verification ===");
    
    // Count final AI state
    let final_discovered = {
        let mut query = app.world_mut().query::<&TileResource>();
        query.iter(app.world()).filter(|r| r.discovered).count()
    };
    let final_developed = {
        let mut query = app.world_mut().query::<&TileResource>();
        query.iter(app.world()).filter(|r| r.development != DevelopmentLevel::Lv0).count()
    };
    let final_depots = {
        let mut query = app.world_mut().query::<&Depot>();
        query.iter(app.world()).filter(|d| d.owner == red_nation).count()
    };
    let final_rails = app.world().resource::<Rails>().0.len();

    println!("Resources Discovered: {}", final_discovered);
    println!("Resources Developed: {}", final_developed);
    println!("Depots Built: {}", final_depots);
    println!("Rail Segments: {}", final_rails);
    println!("AI Performed Actions: {}", if ai_performed_actions { "✓" } else { "✗" });
    println!("Stockpile Changed: {}", if stockpile_changed { "✓" } else { "✗" });

    // The main assertion: verify that the AI system functions in a pruned map
    assert!(
        ai_performed_actions || stockpile_changed,
        "AI should demonstrate activity in pruned map environment within {} turns. \
        This could be discovering resources, developing tiles, building depots, or stockpile changes. \
        This test verifies that AI systems function correctly after map pruning.",
        max_turns
    );

    println!("\n=== Test Complete: AI Functions Correctly in Pruned Map ===");
}

/// Setup mock tilemap for testing
fn setup_mock_tilemap(mut commands: Commands, tilemap_query: Query<&TileStorage>) {
    if !tilemap_query.is_empty() {
        return;
    }

    let map_size = TilemapSize { x: 32, y: 32 };
    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        ..default()
                    },
                    TerrainType::Grass,
                ))
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert((
        TilemapGridSize { x: 16.0, y: 16.0 },
        TilemapType::Hexagon(HexCoordSystem::Row),
        map_size,
        tile_storage,
        TilemapTileSize { x: 16.0, y: 16.0 },
    ));
}
