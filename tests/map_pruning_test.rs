use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs_tilemap::prelude::*;
use rust_imperialism::economy::nation::NationColor;
use rust_imperialism::map::TerrainType;
use rust_imperialism::map::province::{City, Province};
use rust_imperialism::map::province_setup::{TestMapConfig, prune_to_test_map};
use rust_imperialism::map::province_setup::{
    assign_provinces_to_countries, generate_provinces_system,
};
use rust_imperialism::turn_system::TurnPhase;
use rust_imperialism::ui::menu::AppState;
use rust_imperialism::ui::mode::GameMode;

#[test]
fn test_map_pruning_to_red_nation() {
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

    // Adding only the systems we need to test map generation and pruning
    app.add_systems(
        OnEnter(AppState::InGame),
        (
            setup_mock_tilemap,
            ApplyDeferred,
            generate_provinces_system,
            ApplyDeferred,
            assign_provinces_to_countries,
            ApplyDeferred,
            prune_to_test_map,
            ApplyDeferred,
        )
            .chain(),
    );

    // Add the test configuration to trigger pruning
    app.insert_resource(TestMapConfig);

    // Run updates
    for _ in 0..10 {
        app.update();
    }

    let world = app.world_mut();

    // 1. Verify Red nation exists and is the only one
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

        assert_eq!(red_nations.len(), 1, "Should have exactly one Red nation");
        let all_nations: Vec<Entity> = nations_query.iter(world).map(|(e, _)| e).collect();
        assert_eq!(all_nations.len(), 1, "Only Red nation should remain");
        red_nations[0]
    };

    // 2. Verify Provinces belong to Red nation
    let mut provinces_query = world.query::<&Province>();
    let mut total_provinces = 0;
    for province in provinces_query.iter(world) {
        assert_eq!(province.owner, Some(red_nation));
        total_provinces += 1;
    }
    assert!(total_provinces > 0, "Should have kept some provinces");

    // 3. Verify Cities belong to Red nation and capital exists
    let cities: Vec<City> = world.query::<&City>().iter(world).cloned().collect();

    let mut city_count = 0;
    let mut capital_count = 0;
    for city in cities {
        // Find the province for this city
        let mut provinces_query = world.query::<&Province>();
        let province = provinces_query
            .iter(world)
            .find(|p| p.id == city.province)
            .expect("City should have a matching province");

        assert_eq!(
            province.owner,
            Some(red_nation),
            "City province should be owned by Red nation"
        );
        city_count += 1;
        if city.is_capital {
            capital_count += 1;
        }
    }
    assert!(city_count > 0, "Should have kept some cities");
    assert_eq!(capital_count, 1, "Should have exactly one Red capital");
}

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
