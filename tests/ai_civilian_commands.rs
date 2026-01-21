//! Integration tests for AI civilian command validation and execution.

use rust_imperialism::economy::nation::Nation;

#[test]
fn test_ai_move_command_executes() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::civilians::systems::{execute_move_orders, handle_civilian_commands};
    use rust_imperialism::civilians::{
        Civilian, CivilianCommand, CivilianKind, CivilianOrder, CivilianOrderKind,
    };
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::messages::civilians::CivilianCommandRejected;
    use rust_imperialism::turn_system::TurnCounter;

    let mut world = World::new();
    world.init_resource::<TurnCounter>();

    // Register observers
    world.add_observer(handle_civilian_commands);

    // Track rejections
    #[derive(Resource, Default)]
    struct Rejections(Vec<CivilianCommandRejected>);
    world.init_resource::<Rejections>();
    world.add_observer(
        |trigger: On<CivilianCommandRejected>, mut rejections: ResMut<Rejections>| {
            rejections.0.push(*trigger.event());
        },
    );

    // Owned province and tiles
    let nation = world.spawn(Nation).id();
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let start = TilePos { x: 1, y: 1 };
    let target = TilePos { x: 1, y: 2 };
    let map_size = TilemapSize { x: 4, y: 4 };
    let mut storage = TileStorage::empty(map_size);
    let start_tile = world.spawn(TileProvince { province_id }).id();
    let target_tile = world.spawn(TileProvince { province_id }).id();
    storage.set(&start, start_tile);
    storage.set(&target, target_tile);
    world.spawn((storage, map_size));

    let civilian = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: start,
            owner: nation,
            civilian_id: rust_imperialism::civilians::CivilianId(0),
            has_moved: false,
        })
        .id();

    // Trigger the command
    world.trigger(CivilianCommand {
        civilian,
        order: CivilianOrderKind::Move { to: target },
    });
    world.flush();

    assert!(world.get::<CivilianOrder>(civilian).is_some());

    let rejections = world.resource::<Rejections>();
    assert!(rejections.0.is_empty());

    let _ = world.run_system_once(execute_move_orders);
    world.flush();

    let civilian_state = world.get::<Civilian>(civilian).unwrap();
    assert_eq!(civilian_state.position, target);
    assert!(civilian_state.has_moved);
    assert!(world.get::<CivilianOrder>(civilian).is_none());
}

#[test]
fn test_illegal_rail_command_rejected() {
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::civilians::systems::handle_civilian_commands;
    use rust_imperialism::civilians::{
        Civilian, CivilianCommand, CivilianKind, CivilianOrder, CivilianOrderKind,
    };
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::messages::civilians::{CivilianCommandError, CivilianCommandRejected};
    use rust_imperialism::turn_system::TurnCounter;

    let mut world = World::new();
    world.init_resource::<TurnCounter>();

    // Register observers
    world.add_observer(handle_civilian_commands);

    // Track rejections
    #[derive(Resource, Default)]
    struct Rejections(Vec<CivilianCommandRejected>);
    world.init_resource::<Rejections>();
    world.add_observer(
        |trigger: On<CivilianCommandRejected>, mut rejections: ResMut<Rejections>| {
            rejections.0.push(*trigger.event());
        },
    );

    let player = world.spawn(Nation).id();
    let other = world.spawn_empty().id();

    let player_province = ProvinceId(1);
    let other_province = ProvinceId(2);
    world.spawn(Province {
        id: player_province,
        owner: Some(player),
        tiles: vec![],
        city_tile: TilePos { x: 0, y: 0 },
    });
    world.spawn(Province {
        id: other_province,
        owner: Some(other),
        tiles: vec![],
        city_tile: TilePos { x: 3, y: 3 },
    });

    let start = TilePos { x: 2, y: 2 };
    let target = TilePos { x: 2, y: 3 };
    let map_size = TilemapSize { x: 5, y: 5 };
    let mut storage = TileStorage::empty(map_size);
    let start_tile = world
        .spawn(TileProvince {
            province_id: player_province,
        })
        .id();
    let target_tile = world
        .spawn(TileProvince {
            province_id: other_province,
        })
        .id();
    storage.set(&start, start_tile);
    storage.set(&target, target_tile);
    world.spawn((storage, map_size));

    let engineer = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: start,
            owner: player,
            civilian_id: rust_imperialism::civilians::CivilianId(0),
            has_moved: false,
        })
        .id();

    // Trigger the command
    world.trigger(CivilianCommand {
        civilian: engineer,
        order: CivilianOrderKind::BuildRail { to: target },
    });
    world.flush();

    assert!(world.get::<CivilianOrder>(engineer).is_none());

    let rejections = world.resource::<Rejections>();
    assert_eq!(rejections.0.len(), 1);
    assert_eq!(
        rejections.0[0].reason,
        CivilianCommandError::TargetTileUnowned
    );
}
