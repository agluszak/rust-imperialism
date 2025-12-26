use crate::civilians::order_validation::validate_command;
use crate::civilians::types::{Civilian, CivilianId, CivilianKind, CivilianOrderKind};
use crate::map::province::{Province, ProvinceId, TileProvince};
use crate::messages::civilians::CivilianCommandError;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

#[test]
fn test_move_order_rejected_if_tile_occupied() {
    let mut world = World::new();
    let map_size = TilemapSize { x: 10, y: 10 };
    let mut storage = TileStorage::empty(map_size);

    // Setup map and province
    let province_id = ProvinceId(1);
    let nation = Entity::PLACEHOLDER; // Mock nation entity

    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 0, y: 1 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let tile0 = world.spawn(TileProvince { province_id }).id();
    let tile1 = world.spawn(TileProvince { province_id }).id();
    storage.set(&TilePos { x: 0, y: 0 }, tile0);
    storage.set(&TilePos { x: 0, y: 1 }, tile1);
    let storage_entity = world.spawn(storage).id();

    // Spawn civilian 1 at (0, 0) - The one moving
    let civilian1 = Civilian {
        kind: CivilianKind::Engineer,
        position: TilePos { x: 0, y: 0 },
        owner: nation,
        civilian_id: CivilianId(1),
        has_moved: false,
    };
    let c1_entity = world.spawn(civilian1).id();

    // Spawn civilian 2 at (0, 1) - The blocker
    let civilian2 = Civilian {
        kind: CivilianKind::Engineer,
        position: TilePos { x: 0, y: 1 },
        owner: nation,
        civilian_id: CivilianId(2),
        has_moved: false,
    };
    world.spawn(civilian2);

    // Prepare validation inputs
    let mut state: SystemState<(
        Query<&TileStorage>,
        Query<&TileProvince>,
        Query<&Province>,
        Query<&Civilian>,
    )> = SystemState::new(&mut world);
    let (storage_query, tile_provinces, provinces, civilians) = state.get(&world);
    let storage = storage_query
        .get(storage_entity)
        .expect("missing tile storage");

    let civilian1_ref = civilians.get(c1_entity).expect("civilian 1 not found");

    // Attempt to move civilian 1 to (0, 1) where civilian 2 is
    let order = CivilianOrderKind::Move {
        to: TilePos { x: 0, y: 1 },
    };

    let result = validate_command(
        civilian1_ref,
        None,
        None,
        &order,
        Some(storage),
        map_size,
        &tile_provinces,
        &provinces,
        &civilians,
    );

    // Should now reject with TargetTileOccupied
    assert_eq!(
        result,
        Err(CivilianCommandError::TargetTileOccupied),
        "Validation should reject move to occupied tile"
    );
}
