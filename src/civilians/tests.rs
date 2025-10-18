use crate::civilians::commands::DeselectCivilian;
use crate::civilians::engineering::execute_engineer_orders;
use crate::civilians::types::{
    Civilian, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind, JobType,
};
use crate::economy::transport::{PlaceImprovement, Rails, ordered_edge};
use crate::map::province::{Province, ProvinceId, TileProvince};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

#[test]
fn test_engineer_does_not_start_job_on_existing_rail() {
    let mut world = World::new();
    world.init_resource::<Rails>();
    world.init_resource::<crate::turn_system::TurnSystem>();

    // Initialize event resources that the system uses
    world.init_resource::<Messages<crate::ui::logging::TerminalLogEvent>>();
    world.init_resource::<Messages<PlaceImprovement>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create a nation entity
    let nation = world.spawn_empty().id();

    // Create a province owned by the nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage with tiles for the two positions
    let mut tile_storage = TileStorage::empty(TilemapSize { x: 10, y: 10 });
    let start_pos = TilePos { x: 0, y: 0 };
    let target_pos = TilePos { x: 1, y: 0 };

    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&start_pos, tile1);
    tile_storage.set(&target_pos, tile2);
    world.spawn(tile_storage);

    // Create engineer at (0, 0)
    let engineer = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: start_pos,
                owner: nation,
                selected: false,
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::BuildRail { to: target_pos },
            },
        ))
        .id();

    // Add existing rail between the two positions
    let edge = ordered_edge(start_pos, target_pos);
    world.resource_mut::<Rails>().0.insert(edge);

    // Run the execute_engineer_orders system
    let _ = world.run_system_once(execute_engineer_orders);
    world.flush(); // Apply deferred commands

    // Verify engineer moved to target position
    let civilian = world.get::<Civilian>(engineer).unwrap();
    assert_eq!(
        civilian.position, target_pos,
        "Engineer should have moved to target position"
    );
    assert!(civilian.has_moved, "Engineer should be marked as has_moved");

    // Verify engineer does NOT have a CivilianJob component (no job started)
    assert!(
        world.get::<CivilianJob>(engineer).is_none(),
        "Engineer should NOT have a CivilianJob when rail already exists"
    );

    // Verify order was removed
    assert!(
        world.get::<CivilianOrder>(engineer).is_none(),
        "CivilianOrder should be removed after execution"
    );
}

#[test]
fn test_engineer_starts_job_on_new_rail() {
    let mut world = World::new();
    world.init_resource::<Rails>();
    world.init_resource::<crate::turn_system::TurnSystem>();

    // Initialize event resources that the system uses
    world.init_resource::<Messages<crate::ui::logging::TerminalLogEvent>>();
    world.init_resource::<Messages<PlaceImprovement>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create a nation entity
    let nation = world.spawn_empty().id();

    // Create a province owned by the nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage with tiles for the two positions
    let mut tile_storage = TileStorage::empty(TilemapSize { x: 10, y: 10 });
    let start_pos = TilePos { x: 0, y: 0 };
    let target_pos = TilePos { x: 1, y: 0 };

    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&start_pos, tile1);
    tile_storage.set(&target_pos, tile2);
    world.spawn(tile_storage);

    // Create engineer at (0, 0)
    let engineer = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: start_pos,
                owner: nation,
                selected: false,
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::BuildRail { to: target_pos },
            },
        ))
        .id();

    // DO NOT add existing rail (rail doesn't exist)

    // Run the execute_engineer_orders system
    let _ = world.run_system_once(execute_engineer_orders);
    world.flush(); // Apply deferred commands

    // Verify engineer moved to target position
    let civilian = world.get::<Civilian>(engineer).unwrap();
    assert_eq!(
        civilian.position, target_pos,
        "Engineer should have moved to target position"
    );
    assert!(civilian.has_moved, "Engineer should be marked as has_moved");

    // Verify engineer DOES have a CivilianJob component (job started)
    let job = world.get::<CivilianJob>(engineer);
    assert!(
        job.is_some(),
        "Engineer SHOULD have a CivilianJob when building new rail"
    );

    let job = job.unwrap();
    assert_eq!(
        job.job_type,
        JobType::BuildingRail,
        "Job type should be BuildingRail"
    );
    assert_eq!(
        job.turns_remaining, 3,
        "Rail construction should take 3 turns"
    );
    assert_eq!(
        job.target, target_pos,
        "Job target should be the target position"
    );
}
