use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_save::prelude::Save;

/// Unique identifier for a province
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub struct ProvinceId(pub u32);

/// A province is a collection of adjacent tiles with one city
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save)]
pub struct Province {
    pub id: ProvinceId,
    pub tiles: Vec<TilePos>,
    pub city_tile: TilePos,
    pub owner: Option<Entity>, // The country that owns this province
}

/// Marker component for the city within a province
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, MapEntities)]
#[require(Save)]
pub struct City {
    pub province: ProvinceId,
    pub province_entity: Entity,
    pub is_capital: bool,
}

impl MapEntities for City {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.province_entity = mapper.get_mapped(self.province_entity);
    }
}

/// Component that marks a tile as belonging to a province
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct TileProvince {
    pub province_id: ProvinceId,
}

impl Province {
    pub fn new(id: ProvinceId, tiles: Vec<TilePos>, city_tile: TilePos) -> Self {
        Self {
            id,
            tiles,
            city_tile,
            owner: None,
        }
    }
}

impl MapEntities for Province {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(owner) = self.owner.as_mut() {
            *owner = mapper.get_mapped(*owner);
        }
    }
}
