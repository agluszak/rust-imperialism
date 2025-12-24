use crate::civilians::commands::{DeselectCivilian, RescindOrders};
use crate::civilians::engineering::{
    execute_civilian_improvement_orders, execute_engineer_orders, execute_prospector_orders,
};
use crate::civilians::jobs::complete_improvement_jobs;
use crate::civilians::types::{
    Civilian, CivilianId, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind, JobType,
    PreviousPosition, ProspectingKnowledge,
};
use crate::economy::nation::NationId;
use crate::economy::transport::{PlaceImprovement, Rails, ordered_edge};
use crate::map::province::{Province, ProvinceId, TileProvince};
use crate::resources::{DevelopmentLevel, ResourceType, TileResource};
use crate::turn_system::{TurnPhase, TurnSystem};
use bevy::ecs::system::{RunSystemOnce, SystemState};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

#[test]
fn test_engineer_does_not_start_job_on_existing_rail() {
    let mut world = World::new();
    world.init_resource::<Rails>();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();

    // Initialize event resources that the system uses
    world.init_resource::<Messages<PlaceImprovement>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create a nation entity
    let nation = world.spawn(NationId(1)).id();

    // Create a province owned by the nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage with tiles for the two positions
    let map_size = TilemapSize { x: 10, y: 10 };
    let mut tile_storage = TileStorage::empty(map_size);
    let start_pos = TilePos { x: 0, y: 0 };
    let target_pos = TilePos { x: 1, y: 0 };

    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&start_pos, tile1);
    tile_storage.set(&target_pos, tile2);
    world.spawn((tile_storage, map_size));

    // Create engineer at (0, 0)
    let engineer = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: start_pos,
                owner: nation,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
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
    world.init_resource::<ProspectingKnowledge>();

    // Initialize event resources that the system uses
    world.init_resource::<Messages<PlaceImprovement>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create a nation entity
    let nation = world.spawn(NationId(2)).id();

    // Create a province owned by the nation
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    // Create tile storage with tiles for the two positions
    let map_size = TilemapSize { x: 10, y: 10 };
    let mut tile_storage = TileStorage::empty(map_size);
    let start_pos = TilePos { x: 0, y: 0 };
    let target_pos = TilePos { x: 1, y: 0 };

    let tile1 = world.spawn(TileProvince { province_id }).id();
    let tile2 = world.spawn(TileProvince { province_id }).id();
    tile_storage.set(&start_pos, tile1);
    tile_storage.set(&target_pos, tile2);
    world.spawn((tile_storage, map_size));

    // Create engineer at (0, 0)
    let engineer = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: start_pos,
                owner: nation,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
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

#[test]
fn test_prospector_metadata_has_prospect_action() {
    let definition = CivilianKind::Prospector.definition();
    assert_eq!(definition.display_name, "Prospector");
    assert_eq!(definition.orders.len(), 1);
    let order = &definition.orders[0];
    // Check that it's a Prospect order (ignoring the placeholder coordinates)
    assert!(matches!(order.order, CivilianOrderKind::Prospect { .. }));
    assert_eq!(order.execution.job_type(), Some(JobType::Prospecting));
}

#[test]
fn test_only_engineer_requests_orders_panel() {
    assert!(CivilianKind::Engineer.shows_orders_panel());
    assert!(!CivilianKind::Prospector.shows_orders_panel());
    assert!(!CivilianKind::Miner.shows_orders_panel());
    assert!(!CivilianKind::Farmer.shows_orders_panel());
}

#[test]
fn test_default_tile_action_orders() {
    let tile_pos = TilePos { x: 5, y: 10 };
    assert_eq!(
        CivilianKind::Prospector.default_tile_action_order(tile_pos),
        Some(CivilianOrderKind::Prospect { to: tile_pos })
    );
    assert_eq!(
        CivilianKind::Miner.default_tile_action_order(tile_pos),
        Some(CivilianOrderKind::Mine { to: tile_pos })
    );
    assert_eq!(
        CivilianKind::Farmer.default_tile_action_order(tile_pos),
        Some(CivilianOrderKind::ImproveTile { to: tile_pos })
    );
    assert_eq!(
        CivilianKind::Engineer.default_tile_action_order(tile_pos),
        None
    );
}

#[test]
fn test_miner_predicate_accepts_minerals_only() {
    let predicate = CivilianKind::Miner
        .improvement_predicate()
        .expect("Miner should have improvement predicate");

    let mut coal = TileResource::hidden_mineral(ResourceType::Coal);
    coal.discovered = true;
    assert!(predicate(&coal));

    let timber = TileResource::visible(ResourceType::Timber);
    assert!(!predicate(&timber));
}

#[test]
fn test_miner_supports_mine_order() {
    let tile_pos = TilePos { x: 0, y: 0 };
    assert!(
        CivilianKind::Miner.supports_order(&CivilianOrderKind::Mine { to: tile_pos }),
        "Miner should support Mine order"
    );
    assert!(
        !CivilianKind::Miner.supports_order(&CivilianOrderKind::BuildDepot),
        "Miner should not support BuildDepot"
    );
}

#[test]
fn test_prospector_starts_prospecting_job() {
    let mut world = World::new();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();

    world.init_resource::<Messages<DeselectCivilian>>();

    let nation = world.spawn(NationId(3)).id();
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let map_size = TilemapSize { x: 3, y: 3 };
    let mut tile_storage = TileStorage::empty(map_size);
    let tile_pos = TilePos { x: 0, y: 0 };
    let tile_entity = world
        .spawn((
            TileProvince { province_id },
            crate::map::PotentialMineral::new(Some(ResourceType::Coal)),
        ))
        .id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn((tile_storage, map_size));

    let prospector = world
        .spawn((
            Civilian {
                kind: CivilianKind::Prospector,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Prospect { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_prospector_orders);
    world.flush();

    let job = world
        .get::<CivilianJob>(prospector)
        .expect("Prospector should have job");
    assert_eq!(job.job_type, JobType::Prospecting);
    assert_eq!(job.turns_remaining, JobType::Prospecting.duration());

    // PotentialMineral should still exist until prospecting completes
    assert!(
        world
            .get::<crate::map::PotentialMineral>(tile_entity)
            .is_some(),
        "PotentialMineral should remain until job completes"
    );
}

#[test]
fn test_prospecting_job_reveals_resource_on_completion() {
    let mut world = World::new();
    world.init_resource::<ProspectingKnowledge>();

    let mut tile_storage = TileStorage::empty(TilemapSize { x: 3, y: 3 });
    let tile_pos = TilePos { x: 0, y: 0 };
    let tile_entity = world
        .spawn((
            crate::map::PotentialMineral::new(Some(ResourceType::Coal)),
            TileProvince {
                province_id: ProvinceId(1),
            },
        ))
        .id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn(tile_storage);

    let owner = world.spawn(NationId(4)).id();

    let prospector = world
        .spawn((
            Civilian {
                kind: CivilianKind::Prospector,
                position: tile_pos,
                owner,
                owner_id: NationId(4),
                civilian_id: CivilianId(0),
                has_moved: true,
            },
            CivilianJob {
                job_type: JobType::Prospecting,
                turns_remaining: 0,
                target: tile_pos,
            },
        ))
        .id();

    let _ = world.run_system_once(complete_improvement_jobs);

    // After prospecting completes, should have TileResource and ProspectedMineral
    let resource = world
        .get::<TileResource>(tile_entity)
        .expect("Tile should have resource after prospecting");
    assert_eq!(resource.resource_type, ResourceType::Coal);
    assert!(resource.discovered, "Resource should be discovered");

    let prospected = world
        .get::<crate::map::ProspectedMineral>(tile_entity)
        .expect("Tile should have ProspectedMineral marker");
    assert_eq!(prospected.resource_type, ResourceType::Coal);

    // PotentialMineral should be removed
    assert!(
        world
            .get::<crate::map::PotentialMineral>(tile_entity)
            .is_none(),
        "PotentialMineral should be removed after prospecting"
    );

    assert!(
        world.get::<CivilianJob>(prospector).is_none(),
        "complete_improvement_jobs should remove job components after completion"
    );
}

#[test]
fn miner_requires_discovery_before_mining() {
    let mut world = World::new();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();
    world.init_resource::<Messages<DeselectCivilian>>();

    let nation = world.spawn(NationId(5)).id();
    let province_id = ProvinceId(5);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let mut tile_storage = TileStorage::empty(TilemapSize { x: 3, y: 3 });
    let tile_pos = TilePos { x: 0, y: 0 };
    let tile_entity = world
        .spawn((
            TileProvince { province_id },
            TileResource::hidden_mineral(ResourceType::Coal),
        ))
        .id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn(tile_storage);

    let miner = world
        .spawn((
            Civilian {
                kind: CivilianKind::Miner,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Mine { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_civilian_improvement_orders);
    world.flush();

    assert!(
        world.get::<CivilianJob>(miner).is_none(),
        "Miner should not start a job on undiscovered deposits"
    );

    let civilian = world.get::<Civilian>(miner).unwrap();
    assert!(
        !civilian.has_moved,
        "Miner should remain ready to act after failing to start work"
    );

    let resource = world
        .get::<TileResource>(tile_entity)
        .expect("Tile should retain its resource");
    assert!(
        !resource.discovered,
        "Prospecting should still be required before mining"
    );
}

#[test]
fn new_owner_must_reprospect_before_mining() {
    let mut world = World::new();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();
    world.init_resource::<Messages<DeselectCivilian>>();

    let nation_a = world.spawn(NationId(6)).id();
    let nation_b = world.spawn(NationId(7)).id();
    let province_id = ProvinceId(42);
    let province_entity = world
        .spawn(Province {
            id: province_id,
            owner: Some(nation_a),
            tiles: vec![TilePos { x: 0, y: 0 }],
            city_tile: TilePos { x: 0, y: 0 },
        })
        .id();

    let map_size = TilemapSize { x: 3, y: 3 };
    let mut tile_storage = TileStorage::empty(map_size);
    let tile_pos = TilePos { x: 0, y: 0 };
    let tile_entity = world
        .spawn((
            TileProvince { province_id },
            crate::map::PotentialMineral::new(Some(ResourceType::Coal)),
        ))
        .id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn((tile_storage, map_size));

    let prospector = world
        .spawn((
            Civilian {
                kind: CivilianKind::Prospector,
                position: tile_pos,
                owner: nation_a,
                owner_id: NationId(6),
                civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Prospect { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_prospector_orders);
    world.flush();

    {
        let mut job = world
            .get_mut::<CivilianJob>(prospector)
            .expect("Prospector should have started a job");
        job.turns_remaining = 0;
    }

    let _ = world.run_system_once(complete_improvement_jobs);

    {
        let knowledge = world.resource::<ProspectingKnowledge>();
        assert!(
            knowledge.is_discovered_by(tile_entity, nation_a),
            "Original owner should record the discovery"
        );
        assert!(
            !knowledge.is_discovered_by(tile_entity, nation_b),
            "New owner should not have discovery yet"
        );
    }

    assert!(
        world
            .get::<TileResource>(tile_entity)
            .expect("Tile resource component")
            .discovered,
        "Prospecting should reveal the deposit globally"
    );

    world
        .get_mut::<Province>(province_entity)
        .expect("Province component")
        .owner = Some(nation_b);

    let miner = world
        .spawn((
            Civilian {
                kind: CivilianKind::Miner,
                position: tile_pos,
                owner: nation_b,
                owner_id: NationId(7),
                civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Mine { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_civilian_improvement_orders);
    world.flush();

    assert!(
        world.get::<CivilianJob>(miner).is_none(),
        "New owner should not be able to start mining without prospecting"
    );
}

#[test]
fn test_cannot_assign_order_if_order_already_exists() {
    use crate::civilians::order_validation::validate_command;
    use crate::messages::civilians::CivilianCommandError;

    let mut world = World::new();
    let map_size = TilemapSize { x: 4, y: 4 };
    let mut storage = TileStorage::empty(map_size);
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(Entity::PLACEHOLDER),
        tiles: vec![],
        city_tile: TilePos { x: 0, y: 0 },
    });
    let tile_entity = world.spawn(TileProvince { province_id }).id();
    let tile_pos = TilePos { x: 1, y: 1 };
    storage.set(&tile_pos, tile_entity);
    let storage_entity = world.spawn(storage).id();

    let civilian = Civilian {
        kind: CivilianKind::Farmer,
        position: tile_pos,
        owner: Entity::PLACEHOLDER,
        owner_id: NationId(1),
        civilian_id: CivilianId(0),
        has_moved: false,
    };

    // Create an existing order
    let existing_order = CivilianOrder {
        target: CivilianOrderKind::ImproveTile { to: tile_pos },
    };

    let tile_pos = TilePos { x: 1, y: 1 };
    let order = CivilianOrderKind::ImproveTile { to: tile_pos };

    let mut state: SystemState<(Query<&TileStorage>, Query<&TileProvince>, Query<&Province>)> =
        SystemState::new(&mut world);
    let (storage_query, tile_provinces, provinces) = state.get(&world);
    let storage = storage_query
        .get(storage_entity)
        .expect("missing tile storage");

    // Should reject because an order already exists
    let result = validate_command(
        &civilian,
        None,
        Some(&existing_order),
        &order,
        Some(storage),
        map_size,
        &tile_provinces,
        &provinces,
    );

    assert_eq!(
        result,
        Err(CivilianCommandError::AlreadyActed),
        "Should reject when civilian already has an order"
    );
}

#[test]
fn test_can_assign_order_when_no_existing_order() {
    use crate::civilians::order_validation::validate_command;

    let mut world = World::new();
    let map_size = TilemapSize { x: 4, y: 4 };
    let mut storage = TileStorage::empty(map_size);
    let province_id = ProvinceId(1);
    world.spawn(Province {
        id: province_id,
        owner: Some(Entity::PLACEHOLDER),
        tiles: vec![],
        city_tile: TilePos { x: 0, y: 0 },
    });
    let tile_entity = world.spawn(TileProvince { province_id }).id();
    let tile_pos = TilePos { x: 1, y: 1 };
    storage.set(&tile_pos, tile_entity);
    let storage_entity = world.spawn(storage).id();

    let civilian = Civilian {
        kind: CivilianKind::Farmer,
        position: tile_pos,
        owner: Entity::PLACEHOLDER,
        owner_id: NationId(1),
        civilian_id: CivilianId(0),
        has_moved: false,
    };

    let tile_pos = TilePos { x: 1, y: 1 };
    let order = CivilianOrderKind::ImproveTile { to: tile_pos };

    let mut state: SystemState<(Query<&TileStorage>, Query<&TileProvince>, Query<&Province>)> =
        SystemState::new(&mut world);
    let (storage_query, tile_provinces, provinces) = state.get(&world);
    let storage = storage_query
        .get(storage_entity)
        .expect("missing tile storage");

    // Should succeed because no existing order or job
    let result = validate_command(
        &civilian,
        None,
        None,
        &order,
        Some(storage),
        map_size,
        &tile_provinces,
        &provinces,
    );

    assert!(
        result.is_ok(),
        "Should allow order when no existing order or job"
    );
}

#[test]
fn test_rescind_orders_removes_civilian_order_component() {
    use crate::civilians::systems::handle_rescind_orders;
    use crate::civilians::types::{ActionTurn, PreviousPosition};
    use crate::economy::treasury::Treasury;

    let mut world = World::new();

    // Setup turn system
    world.insert_resource(TurnSystem {
        current_turn: 1,
        phase: TurnPhase::PlayerTurn,
        last_job_processing_turn: 0,
    });

    // Initialize message resources
    world.init_resource::<Messages<RescindOrders>>();

    // Create a nation with treasury
    let nation_id = NationId(8);
    let nation = world.spawn((nation_id, Treasury::new(1000))).id();

    // Create a civilian with an order and previous position
    let tile_pos = TilePos { x: 5, y: 5 };
    let prev_pos = TilePos { x: 4, y: 4 };
    let civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Farmer,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
                civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::ImproveTile { to: tile_pos },
            },
            PreviousPosition(prev_pos),
            ActionTurn(1),
        ))
        .id();

    // Send rescind orders message
    {
        let mut state: SystemState<MessageWriter<RescindOrders>> = SystemState::new(&mut world);
        let mut writer = state.get_mut(&mut world);
        writer.write(RescindOrders {
            entity: civilian_entity,
        });
    }

    // Run the rescind orders system
    let _ = world.run_system_once(handle_rescind_orders);

    // Verify the CivilianOrder component was removed
    assert!(
        world.get::<CivilianOrder>(civilian_entity).is_none(),
        "CivilianOrder should be removed after rescinding"
    );

    // Verify other components were also removed
    assert!(
        world.get::<PreviousPosition>(civilian_entity).is_none(),
        "PreviousPosition should be removed after rescinding"
    );
    assert!(
        world.get::<ActionTurn>(civilian_entity).is_none(),
        "ActionTurn should be removed after rescinding"
    );

    // Verify the civilian was restored to previous position
    let civilian = world.get::<Civilian>(civilian_entity).unwrap();
    assert_eq!(
        civilian.position, prev_pos,
        "Civilian should be at previous position"
    );
    assert!(
        !civilian.has_moved,
        "has_moved should be false after rescinding"
    );
}

#[test]
fn test_rescind_orders_removes_civilian_job_and_order() {
    use crate::civilians::systems::handle_rescind_orders;
    use crate::civilians::types::{ActionTurn, CivilianJob, JobType, PreviousPosition};
    use crate::economy::treasury::Treasury;

    let mut world = World::new();

    // Setup turn system
    world.insert_resource(TurnSystem {
        current_turn: 1,
        phase: TurnPhase::PlayerTurn,
        last_job_processing_turn: 0,
    });

    // Initialize message resources
    world.init_resource::<Messages<RescindOrders>>();

    // Create a nation with treasury
    let nation_id = NationId(9);
    let nation = world.spawn((nation_id, Treasury::new(1000))).id();

    // Create a civilian with both a job and an order
    let tile_pos = TilePos { x: 5, y: 5 };
    let prev_pos = TilePos { x: 4, y: 4 };
    let civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
                civilian_id: CivilianId(0),
                has_moved: true,
            },
            CivilianJob {
                job_type: JobType::BuildingRail,
                turns_remaining: 2,
                target: tile_pos,
            },
            CivilianOrder {
                target: CivilianOrderKind::BuildRail { to: tile_pos },
            },
            PreviousPosition(prev_pos),
            ActionTurn(1),
        ))
        .id();

    // Send rescind orders message
    {
        let mut state: SystemState<MessageWriter<RescindOrders>> = SystemState::new(&mut world);
        let mut writer = state.get_mut(&mut world);
        writer.write(RescindOrders {
            entity: civilian_entity,
        });
    }

    // Run the rescind orders system
    let _ = world.run_system_once(handle_rescind_orders);

    // Verify both CivilianJob and CivilianOrder were removed
    assert!(
        world.get::<CivilianJob>(civilian_entity).is_none(),
        "CivilianJob should be removed after rescinding"
    );
    assert!(
        world.get::<CivilianOrder>(civilian_entity).is_none(),
        "CivilianOrder should be removed after rescinding"
    );

    // Verify the civilian state
    let civilian = world.get::<Civilian>(civilian_entity).unwrap();
    assert_eq!(
        civilian.position, prev_pos,
        "Civilian should be at previous position"
    );
    assert!(
        !civilian.has_moved,
        "has_moved should be false after rescinding"
    );

    // Verify refund was given
    let treasury = world.get::<Treasury>(nation).unwrap();
    assert_eq!(treasury.total(), 1050, "Should refund BuildRail cost (50)");
}

#[test]
fn test_skip_turn_removes_order_after_one_turn() {
    use crate::civilians::systems::execute_skip_and_sleep_orders;

    let mut world = World::new();

    let tile_pos = TilePos { x: 5, y: 5 };
    let civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Farmer,
                position: tile_pos,
                owner: Entity::PLACEHOLDER,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::SkipTurn,
            },
        ))
        .id();

    // Execute the skip/sleep system
    let _ = world.run_system_once(execute_skip_and_sleep_orders);

    // Verify the order was removed (SkipTurn is one-time)
    assert!(
        world.get::<CivilianOrder>(civilian_entity).is_none(),
        "SkipTurn order should be removed after execution"
    );

    // Verify civilian was marked as has_moved
    let civilian = world.get::<Civilian>(civilian_entity).unwrap();
    assert!(
        civilian.has_moved,
        "Civilian should be marked as has_moved after skipping"
    );
}

#[test]
fn test_sleep_order_persists_across_turns() {
    use crate::civilians::systems::execute_skip_and_sleep_orders;

    let mut world = World::new();

    let tile_pos = TilePos { x: 5, y: 5 };
    let civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Farmer,
                position: tile_pos,
                owner: Entity::PLACEHOLDER,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Sleep,
            },
        ))
        .id();

    // Execute the skip/sleep system
    let _ = world.run_system_once(execute_skip_and_sleep_orders);

    // Verify the order persists (Sleep continues until rescinded)
    assert!(
        world.get::<CivilianOrder>(civilian_entity).is_some(),
        "Sleep order should persist after execution"
    );

    // Verify civilian was marked as has_moved
    let civilian = world.get::<Civilian>(civilian_entity).unwrap();
    assert!(
        civilian.has_moved,
        "Civilian should be marked as has_moved while sleeping"
    );
}

#[test]
fn test_rescind_wakes_sleeping_civilian() {
    use crate::civilians::systems::handle_rescind_orders;

    let mut world = World::new();
    world.insert_resource(TurnSystem {
        current_turn: 1,
        phase: TurnPhase::PlayerTurn,
        last_job_processing_turn: 0,
    });
    world.init_resource::<Messages<RescindOrders>>();

    let tile_pos = TilePos { x: 5, y: 5 };
    let prev_pos = TilePos { x: 5, y: 5 }; // Same position (no move)
    let civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Farmer,
                position: tile_pos,
                owner: Entity::PLACEHOLDER,
                owner_id: NationId(1),
            civilian_id: CivilianId(0),
                has_moved: true, // Sleeping civilians are marked as moved
            },
            CivilianOrder {
                target: CivilianOrderKind::Sleep,
            },
            PreviousPosition(prev_pos),
        ))
        .id();

    // Send rescind orders message to wake up the civilian
    {
        let mut state: SystemState<MessageWriter<RescindOrders>> = SystemState::new(&mut world);
        let mut writer = state.get_mut(&mut world);
        writer.write(RescindOrders {
            entity: civilian_entity,
        });
    }

    // Run the rescind orders system
    let _ = world.run_system_once(handle_rescind_orders);

    // Verify the Sleep order was removed (civilian is awake)
    assert!(
        world.get::<CivilianOrder>(civilian_entity).is_none(),
        "Sleep order should be removed when rescinded"
    );

    // Verify civilian is no longer marked as has_moved
    let civilian = world.get::<Civilian>(civilian_entity).unwrap();
    assert!(
        !civilian.has_moved,
        "Civilian should be available for action after waking"
    );
}

#[test]
fn miner_respects_max_development_level() {
    let mut world = World::new();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();
    world.init_resource::<Messages<DeselectCivilian>>();

    let nation_id = NationId(10);
    let nation = world.spawn(nation_id).id();
    let province_id = ProvinceId(6);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let mut tile_storage = TileStorage::empty(TilemapSize { x: 3, y: 3 });
    let tile_pos = TilePos { x: 0, y: 0 };
    let mut resource = TileResource::hidden_mineral(ResourceType::Iron);
    resource.discovered = true;
    resource.development = DevelopmentLevel::Lv3;
    let tile_entity = world.spawn((TileProvince { province_id }, resource)).id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn(tile_storage);

    let miner = world
        .spawn((
            Civilian {
                kind: CivilianKind::Miner,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
                civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::Mine { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_civilian_improvement_orders);
    world.flush();

    assert!(
        world.get::<CivilianJob>(miner).is_none(),
        "Miner should not start a job on fully developed deposits"
    );

    let civilian = world.get::<Civilian>(miner).unwrap();
    assert!(
        !civilian.has_moved,
        "Miner should not consume its action on a maxed resource"
    );
}

#[test]
fn farmer_starts_improvement_job_on_visible_resource() {
    let mut world = World::new();
    world.init_resource::<crate::turn_system::TurnSystem>();
    world.init_resource::<ProspectingKnowledge>();
    world.init_resource::<Messages<DeselectCivilian>>();

    let nation_id = NationId(11);
    let nation = world.spawn(nation_id).id();
    let province_id = ProvinceId(7);
    world.spawn(Province {
        id: province_id,
        owner: Some(nation),
        tiles: vec![TilePos { x: 0, y: 0 }],
        city_tile: TilePos { x: 0, y: 0 },
    });

    let map_size = TilemapSize { x: 3, y: 3 };
    let mut tile_storage = TileStorage::empty(map_size);
    let tile_pos = TilePos { x: 0, y: 0 };
    let tile_entity = world
        .spawn((
            TileProvince { province_id },
            TileResource::visible(ResourceType::Grain),
        ))
        .id();
    tile_storage.set(&tile_pos, tile_entity);
    world.spawn((tile_storage, map_size));

    let farmer = world
        .spawn((
            Civilian {
                kind: CivilianKind::Farmer,
                position: tile_pos,
                owner: nation,
                owner_id: NationId(1),
                civilian_id: CivilianId(0),
                has_moved: false,
            },
            CivilianOrder {
                target: CivilianOrderKind::ImproveTile { to: tile_pos },
            },
        ))
        .id();

    let _ = world.run_system_once(execute_civilian_improvement_orders);
    world.flush();

    let job = world
        .get::<CivilianJob>(farmer)
        .expect("Farmer should start an improvement job");
    assert_eq!(job.job_type, JobType::ImprovingTile);
    assert_eq!(job.turns_remaining, JobType::ImprovingTile.duration());

    let civilian = world.get::<Civilian>(farmer).unwrap();
    assert!(
        civilian.has_moved,
        "Farmer should consume its action when starting an improvement"
    );
}
