//! Integration test for AI value-added trading.
//! Verifies that AI can identify profitable opportunities to transform raw materials
//! into finished goods and execute the necessary market and production operations.

mod common;
use common::transition_to_phase;

#[test]
fn test_ai_climbs_value_chain_when_hardware_is_profitable() {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::ai::{AiNation, AiSnapshot, planner::plan_nation};
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
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::menu::AppState;
    use rust_imperialism::ui::mode::GameMode;
    use rust_imperialism::{LogicPlugins, MapLogicPlugin};

    // Create a headless app with minimal plugins
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));

    // Initialize game states
    app.init_state::<TurnPhase>();
    app.insert_state(AppState::InGame);
    app.add_sub_state::<GameMode>();

    // Add LogicPlugins (excluding MapLogic)
    app.add_plugins(LogicPlugins.build().disable::<MapLogicPlugin>());

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

    // Run one turn cycle to build AI snapshot
    app.update(); // Initial update in PlayerTurn

    // Transition to Processing phase, which auto-transitions to EnemyTurn
    // This runs build_ai_snapshot and execute_ai_turn
    transition_to_phase(&mut app, TurnPhase::Processing);

    // Verify AI snapshot was built correctly
    let snapshot = app
        .world()
        .get_resource::<AiSnapshot>()
        .expect("AI snapshot should be created");

    let nation_snapshot = snapshot
        .get_nation(ai_nation)
        .expect("AI nation should be in snapshot");

    // Verify snapshot has correct data
    assert_eq!(
        nation_snapshot.treasury, 10_000,
        "Treasury should be preserved"
    );
    assert!(
        !nation_snapshot.buildings.is_empty(),
        "Buildings should be in snapshot"
    );
    assert!(
        nation_snapshot
            .buildings
            .contains_key(&rust_imperialism::economy::production::BuildingKind::SteelMill),
        "Should have SteelMill"
    );
    assert!(
        nation_snapshot
            .buildings
            .contains_key(&rust_imperialism::economy::production::BuildingKind::MetalWorks),
        "Should have MetalWorks"
    );

    // Verify market prices in snapshot
    assert_eq!(
        snapshot.market.price_for(Good::Iron),
        50,
        "Iron price should be 50"
    );
    assert_eq!(
        snapshot.market.price_for(Good::Coal),
        45,
        "Coal price should be 45"
    );
    assert_eq!(
        snapshot.market.price_for(Good::Steel),
        140,
        "Steel price should be 140"
    );
    assert_eq!(
        snapshot.market.price_for(Good::Hardware),
        360,
        "Hardware price should be 360"
    );

    // Now verify the AI planner generates correct value-added trade plan
    let plan = plan_nation(nation_snapshot, snapshot);

    // Should buy iron and coal for steel production
    let iron_buys: Vec<_> = plan
        .market_buys
        .iter()
        .filter(|(good, _)| *good == Good::Iron)
        .collect();
    let coal_buys: Vec<_> = plan
        .market_buys
        .iter()
        .filter(|(good, _)| *good == Good::Coal)
        .collect();

    assert!(
        !iron_buys.is_empty(),
        "AI should plan to buy iron for steel production. market_buys: {:?}",
        plan.market_buys
    );
    assert!(
        !coal_buys.is_empty(),
        "AI should plan to buy coal for steel production. market_buys: {:?}",
        plan.market_buys
    );

    // Should sell hardware
    let hardware_sells: Vec<_> = plan
        .market_sells
        .iter()
        .filter(|(good, _)| *good == Good::Hardware)
        .collect();

    assert!(
        !hardware_sells.is_empty(),
        "AI should plan to sell manufactured hardware. market_sells: {:?}",
        plan.market_sells
    );

    // Should plan steel production
    let steel_orders: Vec<_> = plan
        .production_orders
        .iter()
        .filter(|o| o.output == Good::Steel)
        .collect();

    assert!(
        !steel_orders.is_empty(),
        "AI should plan steel production from iron and coal. production_orders: {:?}",
        plan.production_orders
    );

    // Should plan hardware production
    let hardware_orders: Vec<_> = plan
        .production_orders
        .iter()
        .filter(|o| o.output == Good::Hardware)
        .collect();

    assert!(
        !hardware_orders.is_empty(),
        "AI should plan hardware production from steel. production_orders: {:?}",
        plan.production_orders
    );
}
