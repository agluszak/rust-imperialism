use bevy::prelude::*;
use bevy::ecs::system::SystemState;
use bevy_ecs_tilemap::prelude::*;
use crate::economy::transport::input::is_ocean_tile;
use crate::map::tiles::TerrainType;

fn setup_app() -> (App, Entity) {
    let mut app = App::new();
    let tile_storage_entity = app.world_mut().spawn_empty().id();
    let tile_storage = TileStorage::empty(TilemapSize { x: 10, y: 10 });
    app.world_mut().entity_mut(tile_storage_entity).insert(tile_storage);
    (app, tile_storage_entity)
}

fn set_tile(world: &mut World, storage_entity: Entity, pos: TilePos, terrain: TerrainType) {
    let id = world.spawn(terrain).id();
    world.entity_mut(storage_entity)
        .get_mut::<TileStorage>()
        .unwrap()
        .set(&pos, id);
}

#[test]
fn test_is_ocean_tile_open_ocean() {
    let (mut app, storage_entity) = setup_app();

    // Center tile (5,5) is Water
    let center = TilePos { x: 5, y: 5 };
    set_tile(app.world_mut(), storage_entity, center, TerrainType::Water);

    // Surround with Water (6 neighbors)
    let neighbors = vec![
        TilePos { x: 6, y: 5 }, TilePos { x: 6, y: 4 }, TilePos { x: 5, y: 4 },
        TilePos { x: 4, y: 5 }, TilePos { x: 4, y: 6 }, TilePos { x: 5, y: 6 }
    ];
    for n in neighbors {
        set_tile(app.world_mut(), storage_entity, n, TerrainType::Water);
    }

    let mut system_state: SystemState<(Query<&TileStorage>, Query<&TerrainType>)> = SystemState::new(app.world_mut());
    let (storage_query, tile_query) = system_state.get(app.world());
    let storage = storage_query.single().unwrap();

    // Should be Ocean
    assert!(is_ocean_tile(center, storage, &tile_query), "Open ocean should be detected as ocean");
}

#[test]
fn test_is_ocean_tile_river() {
    let (mut app, storage_entity) = setup_app();

    // Center tile (5,5) is Water
    let center = TilePos { x: 5, y: 5 };
    set_tile(app.world_mut(), storage_entity, center, TerrainType::Water);

    // Only 2 neighbors are Water (River channel)
    // (6,5) and (4,5) are Water (East and West)
    set_tile(app.world_mut(), storage_entity, TilePos { x: 6, y: 5 }, TerrainType::Water);
    set_tile(app.world_mut(), storage_entity, TilePos { x: 4, y: 5 }, TerrainType::Water);

    // Others are Land (Grass)
    let land_neighbors = vec![
        TilePos { x: 6, y: 4 }, TilePos { x: 5, y: 4 },
        TilePos { x: 4, y: 6 }, TilePos { x: 5, y: 6 }
    ];
    for n in land_neighbors {
        set_tile(app.world_mut(), storage_entity, n, TerrainType::Grass);
    }

    let mut system_state: SystemState<(Query<&TileStorage>, Query<&TerrainType>)> = SystemState::new(app.world_mut());
    let (storage_query, tile_query) = system_state.get(app.world());
    let storage = storage_query.single().unwrap();

    // Should be River (not Ocean)
    assert!(!is_ocean_tile(center, storage, &tile_query), "Straight river should NOT be ocean");
}

#[test]
fn test_is_ocean_tile_coast() {
    let (mut app, storage_entity) = setup_app();

    // Center tile (5,5) is Water
    let center = TilePos { x: 5, y: 5 };
    set_tile(app.world_mut(), storage_entity, center, TerrainType::Water);

    // 3 contiguous neighbors are Water (Coast)
    // E, NE, NW
    set_tile(app.world_mut(), storage_entity, TilePos { x: 6, y: 5 }, TerrainType::Water);
    set_tile(app.world_mut(), storage_entity, TilePos { x: 6, y: 4 }, TerrainType::Water);
    set_tile(app.world_mut(), storage_entity, TilePos { x: 5, y: 4 }, TerrainType::Water);

    // Others are Land
    set_tile(app.world_mut(), storage_entity, TilePos { x: 4, y: 5 }, TerrainType::Grass); // W
    set_tile(app.world_mut(), storage_entity, TilePos { x: 4, y: 6 }, TerrainType::Grass); // SW
    set_tile(app.world_mut(), storage_entity, TilePos { x: 5, y: 6 }, TerrainType::Grass); // SE

    let mut system_state: SystemState<(Query<&TileStorage>, Query<&TerrainType>)> = SystemState::new(app.world_mut());
    let (storage_query, tile_query) = system_state.get(app.world());
    let storage = storage_query.single().unwrap();

    // Should be Ocean (Coast)
    assert!(is_ocean_tile(center, storage, &tile_query), "Coast should be detected as ocean");
}

#[test]
fn test_is_ocean_tile_confluence() {
    let (mut app, storage_entity) = setup_app();

    // Center tile (5,5) is Water
    let center = TilePos { x: 5, y: 5 };
    set_tile(app.world_mut(), storage_entity, center, TerrainType::Water);

    // 3 separated neighbors are Water (Confluence)
    // E (6,5), NW (5,4), SW (4,6)
    set_tile(app.world_mut(), storage_entity, TilePos { x: 6, y: 5 }, TerrainType::Water); // E
    set_tile(app.world_mut(), storage_entity, TilePos { x: 6, y: 4 }, TerrainType::Grass); // NE
    set_tile(app.world_mut(), storage_entity, TilePos { x: 5, y: 4 }, TerrainType::Water); // NW
    set_tile(app.world_mut(), storage_entity, TilePos { x: 4, y: 5 }, TerrainType::Grass); // W
    set_tile(app.world_mut(), storage_entity, TilePos { x: 4, y: 6 }, TerrainType::Water); // SW
    set_tile(app.world_mut(), storage_entity, TilePos { x: 5, y: 6 }, TerrainType::Grass); // SE

    let mut system_state: SystemState<(Query<&TileStorage>, Query<&TerrainType>)> = SystemState::new(app.world_mut());
    let (storage_query, tile_query) = system_state.get(app.world());
    let storage = storage_query.single().unwrap();

    // Should be River (Confluence)
    assert!(!is_ocean_tile(center, storage, &tile_query), "River confluence should NOT be ocean");
}
