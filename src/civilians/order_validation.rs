use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use crate::civilians::types::{Civilian, CivilianJob, CivilianKind, CivilianOrderKind};
use crate::map::province::{Province, TileProvince};
use crate::messages::civilians::CivilianCommandError;

pub fn tile_owned_by_nation(
    tile_pos: TilePos,
    nation_entity: Entity,
    tile_storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> bool {
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
    order: &CivilianOrderKind,
    tile_storage: Option<&TileStorage>,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> Result<(), CivilianCommandError> {
    if job.is_some() {
        return Err(CivilianCommandError::AlreadyHasJob);
    }

    if civilian.has_moved {
        return Err(CivilianCommandError::AlreadyActed);
    }

    let storage = tile_storage.ok_or(CivilianCommandError::MissingTileStorage)?;

    match order {
        CivilianOrderKind::Move { to } => {
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            if !tile_owned_by_nation(*to, civilian.owner, storage, tile_provinces, provinces) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::BuildRail { to } => {
            require_engineer(civilian)?;
            storage
                .get(to)
                .ok_or(CivilianCommandError::MissingTargetTile(*to))?;
            ensure_current_tile_owned(civilian, storage, tile_provinces, provinces)?;
            if !tile_owned_by_nation(*to, civilian.owner, storage, tile_provinces, provinces) {
                return Err(CivilianCommandError::TargetTileUnowned);
            }
            Ok(())
        }
        CivilianOrderKind::BuildDepot | CivilianOrderKind::BuildPort => {
            require_engineer(civilian)?;
            ensure_current_tile_owned(civilian, storage, tile_provinces, provinces)
        }
        CivilianOrderKind::Prospect => {
            if civilian.kind != CivilianKind::Prospector {
                return Err(CivilianCommandError::RequiresProspector);
            }
            ensure_current_tile_owned(civilian, storage, tile_provinces, provinces)
        }
        CivilianOrderKind::Mine => {
            if civilian.kind != CivilianKind::Miner {
                return Err(CivilianCommandError::RequiresImprover);
            }
            ensure_current_tile_owned(civilian, storage, tile_provinces, provinces)
        }
        CivilianOrderKind::ImproveTile
        | CivilianOrderKind::BuildFarm
        | CivilianOrderKind::BuildOrchard => {
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
            ensure_current_tile_owned(civilian, storage, tile_provinces, provinces)
        }
    }
}

fn ensure_current_tile_owned(
    civilian: &Civilian,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
) -> Result<(), CivilianCommandError> {
    if !tile_owned_by_nation(
        civilian.position,
        civilian.owner,
        storage,
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
    use super::*;
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{Entity, World};
    use bevy_ecs_tilemap::prelude::{TileStorage, TilemapSize};

    use crate::map::province::{Province, ProvinceId, TileProvince};

    #[test]
    fn rejects_mismatched_kind() {
        let mut world = World::new();
        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });
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
            selected: false,
            has_moved: false,
        };

        let order = CivilianOrderKind::BuildDepot;

        let mut state: SystemState<(Query<&TileStorage>, Query<&TileProvince>, Query<&Province>)> =
            SystemState::new(&mut world);
        let (storage_query, tile_provinces, provinces) = state.get(&mut world);
        let storage = storage_query
            .get(storage_entity)
            .expect("missing tile storage");

        let result = validate_command(
            &civilian,
            None,
            &order,
            Some(storage),
            &tile_provinces,
            &provinces,
        );

        assert_eq!(result, Err(CivilianCommandError::RequiresEngineer));
    }
}
