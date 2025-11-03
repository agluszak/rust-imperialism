use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

use crate::civilians::order_validation::validate_command;
use crate::civilians::types::{Civilian, CivilianKind, CivilianOrderKind};
use crate::economy::{NationId, PlayerNation};
use crate::map::province::{Province, ProvinceId, TileProvince};
use crate::messages::civilians::CivilianCommandError;

#[test]
fn test_cannot_issue_orders_to_enemy_units() {
    let mut world = World::new();

    // Create player nation
    let player_nation_entity = world.spawn(NationId(1)).id();
    let player_instance =
        moonshine_kind::Instance::<NationId>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create enemy nation
    let enemy_nation_entity = world.spawn(NationId(2)).id();

    // Create a province owned by enemy nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(enemy_nation_entity),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage
    let mut tile_storage = TileStorage::empty(TilemapSize { x: 10, y: 10 });
    let tile1_pos = TilePos { x: 0, y: 0 };
    let tile2_pos = TilePos { x: 1, y: 0 };
    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&tile1_pos, tile1);
    tile_storage.set(&tile2_pos, tile2);
    world.spawn(tile_storage);

    // Create an enemy civilian
    let enemy_civilian = Civilian {
        kind: CivilianKind::Engineer,
        position: tile1_pos,
        owner: enemy_nation_entity,
        selected: false,
        has_moved: false,
    };

    // Try to validate a move command for the enemy unit using SystemState
    let order = CivilianOrderKind::Move { to: tile2_pos };

    let mut system_state: SystemState<(
        Query<&TileStorage>,
        Query<&TileProvince>,
        Query<&Province>,
    )> = SystemState::new(&mut world);

    let (storage_query, tile_prov_query, prov_query) = system_state.get(&world);
    let storage = storage_query.iter().next();

    let result = validate_command(
        &enemy_civilian,
        player_nation_entity, // Player entity
        None,                 // No job
        None,                 // No existing order
        &order,
        storage,
        &tile_prov_query,
        &prov_query,
    );

    // Should fail with NotPlayerOwned error
    assert_eq!(
        result,
        Err(CivilianCommandError::NotPlayerOwned),
        "Should not be able to issue orders to enemy units"
    );
}

#[test]
fn test_can_issue_orders_to_own_units() {
    let mut world = World::new();

    // Create player nation
    let player_nation_entity = world.spawn(NationId(1)).id();
    let player_instance =
        moonshine_kind::Instance::<NationId>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create a province owned by player nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(player_nation_entity),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage
    let mut tile_storage = TileStorage::empty(TilemapSize { x: 10, y: 10 });
    let tile1_pos = TilePos { x: 0, y: 0 };
    let tile2_pos = TilePos { x: 1, y: 0 };
    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&tile1_pos, tile1);
    tile_storage.set(&tile2_pos, tile2);
    world.spawn(tile_storage);

    // Create a player-owned civilian
    let player_civilian = Civilian {
        kind: CivilianKind::Engineer,
        position: tile1_pos,
        owner: player_nation_entity,
        selected: false,
        has_moved: false,
    };

    // Try to validate a move command for the player's own unit using SystemState
    let order = CivilianOrderKind::Move { to: tile2_pos };

    let mut system_state: SystemState<(
        Query<&TileStorage>,
        Query<&TileProvince>,
        Query<&Province>,
    )> = SystemState::new(&mut world);

    let (storage_query, tile_prov_query, prov_query) = system_state.get(&world);
    let storage = storage_query.iter().next();

    let result = validate_command(
        &player_civilian,
        player_nation_entity, // Player entity
        None,                 // No job
        None,                 // No existing order
        &order,
        storage,
        &tile_prov_query,
        &prov_query,
    );

    // Should succeed (ownership check passes, and tile is owned by player)
    assert!(
        result.is_ok(),
        "Should be able to issue orders to own units"
    );
}
