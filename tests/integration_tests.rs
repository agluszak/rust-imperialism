//! Integration tests for rust-imperialism game systems
//!
//! These tests demonstrate ECS testing patterns and verify core game mechanics

use rust_imperialism::economy::nation::NationId;

/// Test turn system functionality
#[test]
fn test_turn_system() {
    use rust_imperialism::turn_system::{TurnPhase, TurnSystem};

    let mut turn_system = TurnSystem::default();
    assert_eq!(turn_system.current_turn, 1);
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);

    // Manually advance phases (since advance_turn was removed)
    turn_system.phase = TurnPhase::Processing;
    assert_eq!(turn_system.phase, TurnPhase::Processing);

    turn_system.phase = TurnPhase::EnemyTurn;
    assert_eq!(turn_system.phase, TurnPhase::EnemyTurn);

    turn_system.phase = TurnPhase::PlayerTurn;
    turn_system.current_turn += 1;
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    assert_eq!(turn_system.current_turn, 2); // New turn
}

/// Test UI state management (simplified)
#[test]
fn test_ui_state() {
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::state::{TurnState, UIState};

    let ui_state = UIState::default();
    // Default display text should reflect default turn state
    assert_eq!(ui_state.turn_display_text(), "Turn: 1 - Player Turn");

    // Test display text generation with a custom turn state
    let ui_state = UIState {
        turn: TurnState {
            current_turn: 5,
            phase: TurnPhase::EnemyTurn,
        },
    };

    assert_eq!(ui_state.turn_display_text(), "Turn: 5 - Enemy Turn");
}

/// Test hex coordinate utilities
#[test]
fn test_hex_coordinates() {
    use bevy_ecs_tilemap::prelude::TilePos;
    use rust_imperialism::map::TilePosExt;

    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 2, y: 1 };

    let hex1 = pos1.to_hex();
    let hex2 = pos2.to_hex();

    let distance = hex1.distance_to(hex2);
    assert_eq!(distance, 1); // Adjacent tiles should have distance 1
}

/// Ensure AI-issued move commands are validated and executed
#[test]
fn test_ai_move_command_executes() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::civilians::systems::{execute_move_orders, handle_civilian_commands};
    use rust_imperialism::civilians::{
        Civilian, CivilianCommand, CivilianKind, CivilianOrder, CivilianOrderKind, DeselectCivilian,
    };
    use rust_imperialism::economy::nation::NationId;
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::messages::civilians::CivilianCommandRejected;
    use rust_imperialism::turn_system::TurnSystem;

    let mut world = World::new();
    world.init_resource::<TurnSystem>();
    world.init_resource::<Messages<CivilianCommand>>();
    world.init_resource::<Messages<CivilianCommandRejected>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Owned province and tiles
    let nation = world.spawn(NationId(1)).id();
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
            owner_id: NationId(1),
            selected: false,
            has_moved: false,
        })
        .id();

    {
        let mut commands = world.resource_mut::<Messages<CivilianCommand>>();
        commands.write(CivilianCommand {
            civilian,
            order: CivilianOrderKind::Move { to: target },
        });
    }

    let _ = world.run_system_once(handle_civilian_commands);
    world.flush();

    assert!(world.get::<CivilianOrder>(civilian).is_some());

    let rejections = world
        .run_system_once(|mut reader: MessageReader<CivilianCommandRejected>| {
            reader.read().cloned().collect::<Vec<_>>()
        })
        .expect("read civilian rejections");
    assert!(rejections.is_empty());

    let _ = world.run_system_once(execute_move_orders);
    world.flush();

    let civilian_state = world.get::<Civilian>(civilian).unwrap();
    assert_eq!(civilian_state.position, target);
    assert!(civilian_state.has_moved);
    assert!(world.get::<CivilianOrder>(civilian).is_none());
}

/// Ensure illegal rail commands are rejected with validation feedback
#[test]
fn test_illegal_rail_command_rejected() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use rust_imperialism::civilians::systems::handle_civilian_commands;
    use rust_imperialism::civilians::{
        Civilian, CivilianCommand, CivilianKind, CivilianOrder, CivilianOrderKind, DeselectCivilian,
    };
    use rust_imperialism::map::province::{Province, ProvinceId, TileProvince};
    use rust_imperialism::messages::civilians::{CivilianCommandError, CivilianCommandRejected};
    use rust_imperialism::turn_system::TurnSystem;

    let mut world = World::new();
    world.init_resource::<TurnSystem>();
    world.init_resource::<Messages<CivilianCommand>>();
    world.init_resource::<Messages<CivilianCommandRejected>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    let player = world.spawn(NationId(2)).id();
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
            owner_id: NationId(2),
            selected: false,
            has_moved: false,
        })
        .id();

    {
        let mut commands = world.resource_mut::<Messages<CivilianCommand>>();
        commands.write(CivilianCommand {
            civilian: engineer,
            order: CivilianOrderKind::BuildRail { to: target },
        });
    }

    let _ = world.run_system_once(handle_civilian_commands);
    world.flush();

    assert!(world.get::<CivilianOrder>(engineer).is_none());

    let rejections = world
        .run_system_once(|mut reader: MessageReader<CivilianCommandRejected>| {
            reader.read().cloned().collect::<Vec<_>>()
        })
        .expect("read civilian rejections");
    assert_eq!(rejections.len(), 1);
    assert_eq!(
        rejections[0].reason,
        CivilianCommandError::TargetTileUnowned
    );
}
