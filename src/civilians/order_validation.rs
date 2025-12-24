use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

use crate::civilians::types::{
    Civilian, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind,
};
use crate::map::province::{Province, TileProvince};
use crate::messages::civilians::CivilianCommandError;

/// Returns true if the tile at `tile_pos` is owned by `nation_entity`.
/// Returns false if the position is out of bounds or not owned.
pub fn tile_owned_by_nation(
    tile_pos: TilePos,
    nation_entity: Entity,
    tile_storage: &TileStorage,
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> bool {
    // Bounds check to prevent panic in TileStorage::get
    if tile_pos.x >= map_size.x || tile_pos.y >= map_size.y {
        return false;
    }

    if let Some(tile_entity) = tile_storage.get(&tile_pos)
        && let Ok(tile_province) = tile_provinces.get(tile_entity)
    {
        for province in provinces.iter() {
            if province.id == tile_province.province_id {
                return province.owner == Some(nation_entity);
            }
        }
    }
    false
}

pub fn validate_command(
    civilian: &Civilian,
    job: Option<&CivilianJob>,
    existing_order: Option<&CivilianOrder>,
    order: &CivilianOrderKind,
    tile_storage: Option<&TileStorage>,
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> Result<(), CivilianCommandError> {
    if job.is_some() {
        return Err(CivilianCommandError::AlreadyHasJob);
    }

    if existing_order.is_some() {
        return Err(CivilianCommandError::AlreadyActed);
    }

    let storage = tile_storage.ok_or(CivilianCommandError::MissingTileStorage)?;

    match order {
        CivilianOrderKind::Move { to } => {
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            if !tile_owned_by_nation(
                *to,
                civilian.owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::BuildRail { to } => {
            require_engineer(civilian)?;
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            ensure_current_tile_owned(civilian, storage, map_size, tile_provinces, provinces)?;
            if !tile_owned_by_nation(
                *to,
                civilian.owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::BuildDepot | CivilianOrderKind::BuildPort => {
            require_engineer(civilian)?;
            ensure_current_tile_owned(civilian, storage, map_size, tile_provinces, provinces)
        }
        CivilianOrderKind::SkipTurn | CivilianOrderKind::Sleep => Ok(()), // No validation needed
        CivilianOrderKind::Prospect { to } => {
            if civilian.kind != CivilianKind::Prospector {
                return Err(CivilianCommandError::RequiresProspector);
            }
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            if !tile_owned_by_nation(
                *to,
                civilian.owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::Mine { to } => {
            if civilian.kind != CivilianKind::Miner {
                return Err(CivilianCommandError::RequiresImprover);
            }
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            if !tile_owned_by_nation(
                *to,
                civilian.owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::ImproveTile { to }
        | CivilianOrderKind::BuildFarm { to }
        | CivilianOrderKind::BuildOrchard { to } => {
            if !matches!(
                civilian.kind,
                CivilianKind::Farmer
                    | CivilianKind::Rancher
                    | CivilianKind::Forester
                    | CivilianKind::Driller
                    | CivilianKind::Miner
            ) {
                return Err(CivilianCommandError::RequiresImprover);
            }
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            if !tile_owned_by_nation(
                *to,
                civilian.owner,
                storage,
                map_size,
                tile_provinces,
                provinces,
            ) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
    }
}

fn ensure_current_tile_owned(
    civilian: &Civilian,
    storage: &TileStorage,
    map_size: TilemapSize,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> Result<(), CivilianCommandError> {
    if !tile_owned_by_nation(
        civilian.position,
        civilian.owner,
        storage,
        map_size,
        tile_provinces,
        provinces,
    ) {
        return Err(CivilianCommandError::CurrentTileUnowned);
    }
    Ok(())
}

fn require_engineer(civilian: &Civilian) -> Result<(), CivilianCommandError> {
    if civilian.kind != CivilianKind::Engineer {
        return Err(CivilianCommandError::RequiresEngineer);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::civilians::*;
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{Entity, World};
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};

    use crate::civilians::order_validation::validate_command;
    use crate::map::province::{Province, ProvinceId, TileProvince};

    #[test]
    fn rejects_mismatched_kind() {
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
            kind: CivilianKind::Prospector,
            position: tile_pos,
            owner: Entity::PLACEHOLDER,
            owner_id: crate::economy::nation::NationId(1),
            civilian_id: CivilianId(0),
            has_moved: false,
        };

        let order = CivilianOrderKind::BuildDepot;

        let mut state: SystemState<(Query<&TileStorage>, Query<&TileProvince>, Query<&Province>)> =
            SystemState::new(&mut world);
        let (storage_query, tile_provinces, provinces) = state.get(&world);
        let storage = storage_query
            .get(storage_entity)
            .expect("missing tile storage");

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

        assert_eq!(result, Err(CivilianCommandError::RequiresEngineer));
    }
}
