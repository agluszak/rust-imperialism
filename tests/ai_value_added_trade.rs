//! Integration test for AI value-added trading.
//! Verifies that AI can identify profitable opportunities to transform raw materials 
//! into finished goods and execute the necessary market and production operations.

mod common;
use common::transition_to_phase;

#[test]
fn test_ai_climbs_value_chain_when_hardware_is_profitable() {
    use bevy::ecs::message::MessageReader;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::ai::{AiNation, AiSnapshot};
    use rust_imperialism::civilians::types::ProspectingKnowledge;
    use rust_imperialism::economy::{
        EconomyPlugin,
        goods::Good,
        nation::{Capital, Nation},
        production::{Buildings, ProductionSettings},
        stockpile::Stockpile,
        technology::Technologies,
        treasury::Treasury,
    };
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::map::tiles::TerrainType;
    use rust_imperialism::messages::{AdjustMarketOrder, AdjustProduction, MarketInterest};
    use rust_imperialism::turn_system::{TurnPhase, TurnSystemPlugin};
    use rust_imperialism::ui::menu::AppState;
    use rust_imperialism::ui::mode::GameMode;

    // Create a headless app with minimal plugins
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Initialize game states
    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    // Add game plugins
    app.add_plugins((
        TurnSystemPlugin,
        EconomyPlugin,
        rust_imperialism::ai::AiPlugin,
        rust_imperialism::civilians::CivilianPlugin,
    ));

    // Initialize required resources
    app.init_resource::<ProspectingKnowledge>();

    // Set up profitable market prices in the MarketPriceModel
    // The AI will read these prices when it builds its snapshot
    let mut market_prices = rust_imperialism::economy::market::MarketPriceModel::default();
    market_prices.set_base_price(Good::Iron, 50);
    market_prices.set_base_price(Good::Coal, 45);
    market_prices.set_base_price(Good::Steel, 140);
    market_prices.set_base_price(Good::Hardware, 360);
    app.insert_resource(market_prices);

    // Create a small test map
    let map_size = TilemapSize { x: 5, y: 5 };
    let mut tile_storage = TileStorage::empty(map_size);
    let capital_pos = TilePos { x: 2, y: 2 };

    let province_id = ProvinceId(1);
    let mut province_tiles = vec![];

    // Create tiles
    for x in 0..5 {
        for y in 0..5 {
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

    // Create AI nation with initial buildings and plenty of money
    let ai_nation = app
        .world_mut()
        .spawn((
            AiNation,
            Nation,
            Capital(capital_pos),
            Stockpile::default(),
            Treasury::new(10_000),
            Technologies::default(),
            Buildings::with_all_initial(),
            ProductionSettings::default(),
        ))
        .id();

    // Create province owned by the AI nation
    app.world_mut().spawn(Province {
        id: province_id,
        owner: Some(ai_nation),
        tiles: province_tiles,
        city_tile: capital_pos,
    });

    println!("\n=== Starting AI Value-Added Trade Integration Test ===");
    println!("Market prices:");
    println!("  Iron: 50, Coal: 45");
    println!("  Steel: 140 (cost: 95, profit: 45)");
    println!("  Hardware: 360 (cost: 280, profit: 80)");

    // Run one turn cycle
    app.update(); // PlayerTurn
    transition_to_phase(&mut app, TurnPhase::Processing);
    transition_to_phase(&mut app, TurnPhase::EnemyTurn);

    // After EnemyTurn, AI should have planned and issued orders
    // Check that AI issued the expected market orders
    let market_orders = app
        .world_mut()
        .run_system_once(|mut reader: MessageReader<AdjustMarketOrder>| {
            reader.read().cloned().collect::<Vec<_>>()
        })
        .unwrap();

    println!("\nMarket orders issued by AI:");
    for order in &market_orders {
        println!(
            "  {:?} {} of {:?}",
            order.kind, order.requested, order.good
        );
    }

    // Should buy iron and coal
    let iron_buys: Vec<_> = market_orders
        .iter()
        .filter(|o| o.good == Good::Iron && matches!(o.kind, MarketInterest::Buy))
        .collect();
    let coal_buys: Vec<_> = market_orders
        .iter()
        .filter(|o| o.good == Good::Coal && matches!(o.kind, MarketInterest::Buy))
        .collect();

    assert!(
        !iron_buys.is_empty(),
        "AI should buy iron for steel production"
    );
    assert!(
        !coal_buys.is_empty(),
        "AI should buy coal for steel production"
    );

    // Should sell hardware
    let hardware_sells: Vec<_> = market_orders
        .iter()
        .filter(|o| o.good == Good::Hardware && matches!(o.kind, MarketInterest::Sell))
        .collect();

    assert!(
        !hardware_sells.is_empty(),
        "AI should sell manufactured hardware"
    );

    // Check production orders
    let production_orders = app
        .world_mut()
        .run_system_once(|mut reader: MessageReader<AdjustProduction>| {
            reader.read().cloned().collect::<Vec<_>>()
        })
        .unwrap();

    println!("\nProduction orders issued by AI:");
    for order in &production_orders {
        println!("  {} units of {:?}", order.target_output, order.output_good);
    }

    // Should plan steel production
    let steel_orders: Vec<_> = production_orders
        .iter()
        .filter(|o| o.output_good == Good::Steel)
        .collect();

    assert!(
        !steel_orders.is_empty(),
        "AI should plan steel production from iron and coal"
    );

    // Should plan hardware production
    let hardware_orders: Vec<_> = production_orders
        .iter()
        .filter(|o| o.output_good == Good::Hardware)
        .collect();

    assert!(
        !hardware_orders.is_empty(),
        "AI should plan hardware production from steel"
    );

    // No longer checking production choice since it's now determined dynamically
    // based on stockpile availability at production time

    // Check AI snapshot to verify planning worked
    if let Some(snapshot) = app.world().get_resource::<AiSnapshot>() {
        if let Some(nation_snapshot) = snapshot.get_nation(ai_nation) {
            println!("\nAI Nation State:");
            println!("  Treasury: {}", nation_snapshot.treasury);
            println!("  Stockpile: {:?}", nation_snapshot.stockpile);
        }
    }

    println!("\n=== Test Complete: AI successfully planned value-added trading ===");
}
