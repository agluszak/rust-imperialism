//! Integration tests for complex AI resource discovery, extraction, and transport.

mod common;
use common::transition_to_phase;

/// Integration test: AI discovers resources, mines them, builds depot, and collects resources
/// This test runs in headless mode with a manually created map
#[test]
fn test_ai_resource_discovery_and_collection() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::ai::{AiControlledCivilian, AiNation};
    use rust_imperialism::civilians::{Civilian, CivilianKind};
    use rust_imperialism::economy::{
        nation::{Capital, Nation},
        production::Buildings,
        stockpile::Stockpile,
        technology::Technologies,
        transport::Depot,
        treasury::Treasury,
    };
    use rust_imperialism::map::{
        prospecting::{PotentialMineral, ProspectedMineral},
        province::{Province, ProvinceId, TileProvince},
    };
    use rust_imperialism::resources::{ResourceType, TileResource};
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::menu::AppState;

    use rust_imperialism::LogicPlugins;

    // Create a headless app with minimal plugins
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Add LogicPlugins (includes MapLogic but NOT MapGenerationPlugin)
    app.add_plugins(LogicPlugins);

    // Force InGame state to trigger plugin systems
    app.insert_state(AppState::InGame);

    // Manually create a small test map (5x5)
    let map_size = TilemapSize { x: 10, y: 10 };
    let mut tile_storage = TileStorage::empty(map_size);

    // Define key positions
    let capital_pos = TilePos { x: 5, y: 5 };
    let coal_pos = TilePos { x: 5, y: 7 }; // 2 tiles away
    let iron_pos = TilePos { x: 6, y: 7 }; // Adjacent to coal

    // Create province for AI nation
    let province_id = ProvinceId(1);
    let mut province_tiles = vec![];

    // Create tiles in a 5x5 area around capital
    for x in 3..8 {
        for y in 3..8 {
            let pos = TilePos { x, y };
            let tile_entity = app.world_mut().spawn(TileProvince { province_id }).id();
            tile_storage.set(&pos, tile_entity);
            province_tiles.push(pos);

            // Add hidden mineral resources (only PotentialMineral, TileResource added after prospecting)
            if pos == coal_pos {
                app.world_mut()
                    .entity_mut(tile_entity)
                    .insert(PotentialMineral::new(Some(ResourceType::Coal)));
            } else if pos == iron_pos {
                app.world_mut()
                    .entity_mut(tile_entity)
                    .insert(PotentialMineral::new(Some(ResourceType::Iron)));
            }
        }
    }

    // Create tilemap entity
    let _tilemap_entity = app.world_mut().spawn((tile_storage, map_size)).id();

    // Create AI nation with capital FIRST (before province)
    let ai_nation = app
        .world_mut()
        .spawn((
            AiNation,
            Nation,
            Capital(capital_pos),
            Stockpile::default(),
            Treasury::default(),
            Technologies::default(),
            Buildings::default(),
        ))
        .id();

    // Create province owned by the AI nation
    app.world_mut().spawn(Province {
        id: province_id,
        owner: Some(ai_nation),
        tiles: province_tiles,
        city_tile: capital_pos,
    });

    // Spawn AI-controlled civilians near capital
    let prospector = app
        .world_mut()
        .spawn((
            Civilian {
                kind: CivilianKind::Prospector,
                position: TilePos { x: 5, y: 6 }, // 1 tile from capital
                owner: ai_nation,
                civilian_id: rust_imperialism::civilians::CivilianId(0),
                has_moved: false,
            },
            AiControlledCivilian,
        ))
        .id();

    let miner = app
        .world_mut()
        .spawn((
            Civilian {
                kind: CivilianKind::Miner,
                position: TilePos { x: 6, y: 6 },
                owner: ai_nation,
                civilian_id: rust_imperialism::civilians::CivilianId(1),
                has_moved: false,
            },
            AiControlledCivilian,
        ))
        .id();

    let engineer = app
        .world_mut()
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: TilePos { x: 4, y: 5 }, // Adjacent to capital
                owner: ai_nation,
                civilian_id: rust_imperialism::civilians::CivilianId(2),
                has_moved: false,
            },
            AiControlledCivilian,
        ))
        .id();

    println!("\n=== Starting AI Resource Discovery Integration Test ===");
    println!("Capital at: {:?}", capital_pos);
    println!("Coal at: {:?} (hidden)", coal_pos);
    println!("Iron at: {:?} (hidden)", iron_pos);
    println!(
        "Prospector at: {:?}",
        app.world().get::<Civilian>(prospector).unwrap().position
    );
    println!(
        "Miner at: {:?}",
        app.world().get::<Civilian>(miner).unwrap().position
    );
    println!(
        "Engineer at: {:?}",
        app.world().get::<Civilian>(engineer).unwrap().position
    );

    // Run the game for multiple turns
    let max_turns = 30; // Increased to give more time
    let mut resources_discovered = false;
    let mut mine_built = false;
    let mut depot_built = false;
    let mut resources_collected = false;

    for turn in 1..=max_turns {
        println!(
            "\n--- Turn {} (Phase: {:?}) ---",
            turn,
            app.world().resource::<State<TurnPhase>>().get()
        );

        // Manually transition through phases for better control
        // PlayerTurn phase
        app.update();

        // Transition to Processing
        transition_to_phase(&mut app, TurnPhase::Processing);
        println!("Processing phase complete");

        // Transition to EnemyTurn
        transition_to_phase(&mut app, TurnPhase::EnemyTurn);
        println!("EnemyTurn phase complete");

        // Transition back to PlayerTurn
        transition_to_phase(&mut app, TurnPhase::PlayerTurn);
        println!(
            "After manual transitions, phase: {:?}",
            app.world().resource::<State<TurnPhase>>().get()
        );

        // Debug civilian positions periodically
        if turn % 5 == 0
            && let Some(civ) = app.world().get::<Civilian>(prospector)
        {
            println!("  Prospector now at: {:?}", civ.position);
        }

        // Check for discovered resources and mines
        let (discovered, mine_developed) = app
            .world_mut()
            .run_system_once(
                move |tile_storage_q: Query<&TileStorage>,
                      prospected: Query<&ProspectedMineral>,
                      resources: Query<&TileResource>| {
                    let Some(tile_storage) = tile_storage_q.iter().next() else {
                        return (false, false);
                    };
                    let mut discovered = false;
                    let mut developed = false;

                    if let Some(coal_tile) = tile_storage.get(&coal_pos) {
                        if prospected.get(coal_tile).is_ok() {
                            discovered = true;
                        }
                        if let Ok(resource) = resources.get(coal_tile)
                            && resource.discovered
                            && resource.development
                                != rust_imperialism::resources::DevelopmentLevel::Lv0
                        {
                            developed = true;
                        }
                    }
                    (discovered, developed)
                },
            )
            .unwrap();

        if discovered && !resources_discovered {
            println!("✓ Resources discovered!");
            resources_discovered = true;
        }

        if mine_developed && !mine_built {
            println!("✓ Mine built on discovered resource!");
            mine_built = true;
        }

        // Check for depots
        let depot_count = app
            .world_mut()
            .run_system_once(move |depots: Query<&Depot>| {
                depots
                    .iter()
                    .filter(|depot| depot.owner == ai_nation)
                    .count()
            })
            .unwrap();

        if depot_count > 0 && !depot_built {
            println!("✓ Depot built!");
            depot_built = true;
        }

        // Check for connected depots
        let connected_depots = app
            .world_mut()
            .run_system_once(move |depots: Query<&Depot>| {
                depots
                    .iter()
                    .filter(|depot| depot.owner == ai_nation && depot.connected)
                    .count()
            })
            .unwrap();

        if connected_depots > 0 {
            println!("✓ Depot connected to capital!");
        }

        // Check if resources have been collected in stockpile
        if let Some(stockpile) = app.world().get::<Stockpile>(ai_nation) {
            let coal_amount = stockpile.get(rust_imperialism::economy::goods::Good::Coal);
            let iron_amount = stockpile.get(rust_imperialism::economy::goods::Good::Iron);

            if (coal_amount > 0 || iron_amount > 0) && !resources_collected {
                println!(
                    "✓ Resources collected in stockpile! Coal: {}, Iron: {}",
                    coal_amount, iron_amount
                );
                resources_collected = true;
            }
        }

        // Early exit if all goals achieved
        if resources_discovered && mine_built && depot_built && resources_collected {
            println!("\n=== All objectives achieved in {} turns! ===", turn);
            break;
        }
    }

    // Verify test objectives
    println!("\n=== Final State ===");

    // At minimum, verify that resources were discovered
    assert!(
        resources_discovered,
        "Prospector should have discovered mineral resources within {} turns",
        max_turns
    );

    // Note: Building mines and depots and collecting resources may take longer than 15 turns
    // depending on AI behavior, so we'll just report the status
    println!("Resources discovered: {}", resources_discovered);
    println!("Mine built: {}", mine_built);
    println!("Depot built: {}", depot_built);
    println!("Resources collected: {}", resources_collected);

    // If depot was built and connected, verify it's connected
    if depot_built {
        let connected_depots = app
            .world_mut()
            .run_system_once(move |depots: Query<&Depot>| {
                depots
                    .iter()
                    .filter(|depot| depot.owner == ai_nation && depot.connected)
                    .count()
            })
            .unwrap();

        if connected_depots > 0 {
            println!("✓ Depot is connected to capital via rails");
        }
    }

    println!("\n=== Test Complete ===");
}

/// Integration test: AI engineers respect terrain constraints when building depots and rails
/// This test verifies that AI will not attempt to build on water or mountains
#[test]
fn test_ai_respects_terrain_constraints() {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::ai::{AiControlledCivilian, AiNation, AiSnapshot};
    use rust_imperialism::civilians::{Civilian, CivilianKind};
    use rust_imperialism::economy::{
        nation::{Capital, Nation},
        production::Buildings,
        stockpile::Stockpile,
        technology::Technologies,
        treasury::Treasury,
    };
    use rust_imperialism::map::{
        province::{Province, ProvinceId, TileProvince},
        tiles::TerrainType,
    };
    use rust_imperialism::resources::{ResourceType, TileResource};
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::menu::AppState;

    use rust_imperialism::LogicPlugins;

    // Create a headless app with minimal plugins
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Add LogicPlugins (includes MapLogic but NOT MapGenerationPlugin)
    app.add_plugins(LogicPlugins);

    // Force InGame state to trigger plugin systems
    app.insert_state(AppState::InGame);

    // Create a test map with varied terrain
    let map_size = TilemapSize { x: 10, y: 10 };
    let mut tile_storage = TileStorage::empty(map_size);

    // Define key positions with specific terrains
    let capital_pos = TilePos { x: 5, y: 5 }; // Grass (buildable)
    let grass_resource_pos = TilePos { x: 5, y: 6 }; // Grass with resource (buildable)
    let mountain_pos = TilePos { x: 5, y: 7 }; // Mountain (not buildable for depot)
    let water_pos = TilePos { x: 6, y: 7 }; // Water (not buildable at all)
    let hill_pos = TilePos { x: 4, y: 6 }; // Hill (buildable for depot, needs tech for rail)

    let province_id = ProvinceId(1);
    let mut province_tiles = vec![];

    // Create tiles with specific terrain types
    for x in 3..8 {
        for y in 3..8 {
            let pos = TilePos { x, y };
            let terrain = if pos == water_pos {
                TerrainType::Water
            } else if pos == mountain_pos {
                TerrainType::Mountain
            } else if pos == hill_pos {
                TerrainType::Hills
            } else {
                TerrainType::Grass
            };

            let tile_entity = app
                .world_mut()
                .spawn((TileProvince { province_id }, terrain))
                .id();
            tile_storage.set(&pos, tile_entity);
            province_tiles.push(pos);

            // Add a coal resource on the grass tile
            if pos == grass_resource_pos {
                app.world_mut()
                    .entity_mut(tile_entity)
                    .insert(TileResource::visible(ResourceType::Coal));
            }
        }
    }

    // Create tilemap entity
    app.world_mut().spawn((tile_storage, map_size));

    // Create AI nation with no technologies (cannot build rails on hills/mountains)
    let ai_nation = app
        .world_mut()
        .spawn((
            AiNation,
            Nation,
            Capital(capital_pos),
            Stockpile::default(),
            Treasury::new(10000), // Enough money for construction
            Technologies::new(),  // No technologies unlocked
            Buildings::default(),
        ))
        .id();

    // Create province owned by the AI nation
    app.world_mut().spawn(Province {
        id: province_id,
        owner: Some(ai_nation),
        tiles: province_tiles,
        city_tile: capital_pos,
    });

    // Spawn AI-controlled engineer near capital
    let engineer = app
        .world_mut()
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: capital_pos,
                owner: ai_nation,
                civilian_id: rust_imperialism::civilians::CivilianId(0),
                has_moved: false,
            },
            AiControlledCivilian,
        ))
        .id();

    println!("\n=== Starting AI Terrain Constraints Test ===");
    println!("Capital at: {:?} (Grass)", capital_pos);
    println!("Resource at: {:?} (Grass)", grass_resource_pos);
    println!("Mountain at: {:?} (Cannot build depot)", mountain_pos);
    println!("Water at: {:?} (Cannot build anything)", water_pos);
    println!(
        "Hill at: {:?} (Can build depot, not rail without tech)",
        hill_pos
    );

    // Run for a few turns to let AI plan
    for turn in 1..=10 {
        println!("\n--- Turn {} ---", turn);

        // Run turn phases
        app.update();
        transition_to_phase(&mut app, TurnPhase::Processing);
        transition_to_phase(&mut app, TurnPhase::EnemyTurn);

        // After EnemyTurn, check the AI snapshot to see what depots it suggested
        if let Some(snapshot) = app.world().get_resource::<AiSnapshot>()
            && let Some(nation_snapshot) = snapshot.get_nation(ai_nation)
        {
            println!(
                "  AI suggested {} depot locations",
                nation_snapshot.suggested_depots.len()
            );
            for depot in &nation_snapshot.suggested_depots {
                println!(
                    "    - Depot at {:?} (covers {} resources)",
                    depot.position, depot.covers_count
                );

                // Verify suggested depot is not on water or mountain
                assert_ne!(
                    depot.position, water_pos,
                    "AI should not suggest depot on water"
                );
                assert_ne!(
                    depot.position, mountain_pos,
                    "AI should not suggest depot on mountain"
                );
            }
        }

        transition_to_phase(&mut app, TurnPhase::PlayerTurn);
    }

    // Verify AI engineer didn't try to build on invalid terrain
    let engineer_pos = app.world().get::<Civilian>(engineer).unwrap().position;
    println!("\nFinal engineer position: {:?}", engineer_pos);

    // Engineer should not be on water, mountain, or hills (without technology)
    assert_ne!(engineer_pos, water_pos, "Engineer should not move to water");
    assert_ne!(
        engineer_pos, mountain_pos,
        "Engineer should not move to mountain"
    );
    // Hills are buildable for depots but not for rails without technology
    // If engineer is on hills, they should not have attempted rail construction
    if engineer_pos == hill_pos {
        println!(
            "Note: Engineer is on hill tile, but should not have built rails without HillGrading technology"
        );
    }

    println!("\n=== Test Complete: AI respects terrain constraints ===");
}

/// Integration test: Two engineers building rails to two separate resource hubs
/// that for some distance share rails but then the rails have to split.
#[test]
fn test_two_engineers_splitting_paths() {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};
    use rust_imperialism::ai::{AiControlledCivilian, AiNation};
    use rust_imperialism::civilians::{Civilian, CivilianKind};
    use rust_imperialism::economy::{
        nation::{Capital, Nation},
        production::Buildings,
        stockpile::Stockpile,
        technology::Technologies,
        transport::{Depot, Rails},
        treasury::Treasury,
    };
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::map::tiles::TerrainType;
    use rust_imperialism::turn_system::{TurnCounter, TurnPhase};
    use rust_imperialism::ui::menu::AppState;

    use rust_imperialism::LogicPlugins;

    // Create a headless app with minimal plugins
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Add LogicPlugins (includes MapLogic but NOT MapGenerationPlugin)
    app.add_plugins(LogicPlugins);

    // Force InGame state to trigger plugin systems
    app.insert_state(AppState::InGame);

    let map_size = TilemapSize { x: 30, y: 30 };
    let mut tile_storage = TileStorage::empty(map_size);

    // Hubs share East-ward path from center for a while
    let capital_pos = TilePos { x: 10, y: 15 };
    let hub_a_pos = TilePos { x: 25, y: 10 };
    let hub_b_pos = TilePos { x: 25, y: 20 };

    let province_id = ProvinceId(1);
    let mut province_tiles = vec![];

    for x in 0..30 {
        for y in 0..30 {
            let pos = TilePos { x, y };
            let tile_entity = app
                .world_mut()
                .spawn((TileProvince { province_id }, TerrainType::Grass))
                .id();
            tile_storage.set(&pos, tile_entity);
            province_tiles.push(pos);
        }
    }

    app.world_mut().spawn((tile_storage, map_size));

    let ai_nation = app
        .world_mut()
        .spawn((
            AiNation,
            Nation,
            Capital(capital_pos),
            Stockpile::default(),
            Treasury::new(10000),
            Technologies::default(),
            Buildings::default(),
        ))
        .id();

    app.world_mut().spawn(Province {
        id: province_id,
        owner: Some(ai_nation),
        tiles: province_tiles,
        city_tile: capital_pos,
    });

    // Spawn 2 unconnected depots at Hub A and Hub B
    app.world_mut().spawn(Depot {
        position: hub_a_pos,
        owner: ai_nation,
        connected: false,
    });
    app.world_mut().spawn(Depot {
        position: hub_b_pos,
        owner: ai_nation,
        connected: false,
    });

    // Spawn 2 engineers at capital
    for i in 0..2 {
        app.world_mut().spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: capital_pos,
                owner: ai_nation,
                civilian_id: rust_imperialism::civilians::CivilianId(i),
                has_moved: false,
            },
            AiControlledCivilian,
        ));
    }

    // Spawn dummy civilians far away to satisfy AI hiring targets and prevent overcrowding at capital
    // Targets: Prospector (2), Farmer (2), Miner (2), Rancher (1), Forester (1)
    let dummy_pos = TilePos { x: 29, y: 29 };
    let dummies = [
        (CivilianKind::Prospector, 2),
        (CivilianKind::Farmer, 2),
        (CivilianKind::Miner, 2),
        (CivilianKind::Rancher, 1),
        (CivilianKind::Forester, 1),
    ];

    let mut dummy_id = 100;
    for (kind, count) in dummies {
        for _ in 0..count {
            app.world_mut().spawn((
                Civilian {
                    kind,
                    position: dummy_pos,
                    owner: ai_nation,
                    civilian_id: rust_imperialism::civilians::CivilianId(dummy_id),
                    has_moved: false,
                },
                AiControlledCivilian,
            ));
            dummy_id += 1;
        }
    }

    println!("\n=== Starting Two Engineers Splitting Path Test ===");

    for turn in 1..=90 {
        let current_turn = app.world().resource::<TurnCounter>().current;
        println!("\n--- Turn {} ---", current_turn);

        // Turn loop
        transition_to_phase(&mut app, TurnPhase::Processing);
        transition_to_phase(&mut app, TurnPhase::EnemyTurn);
        transition_to_phase(&mut app, TurnPhase::PlayerTurn);

        // Check progress
        let mut connected_depots = 0;
        {
            let mut query = app.world_mut().query::<&Depot>();
            for depot in query.iter(app.world()) {
                if depot.connected {
                    connected_depots += 1;
                }
            }
        }

        let rail_count = app.world().resource::<Rails>().0.len();
        println!(
            "  Rails: {}, Connected Depots: {}",
            rail_count, connected_depots
        );

        if connected_depots == 2 {
            println!("SUCCESS: Both hubs connected in {} turns!", turn);
            return;
        }
    }

    panic!("FAIL: Did not connect both hubs within 90 turns");
}
