use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

/// Unique identifier for a province
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProvinceId(pub u32);

/// A province is a collection of adjacent tiles with one city
#[derive(Component, Debug, Clone)]
pub struct Province {
    pub id: ProvinceId,
    pub tiles: Vec<TilePos>,
    pub city_tile: TilePos,
    pub owner: Option<Entity>, // The country that owns this province
}

/// Marker component for the city within a province
#[derive(Component, Debug, Clone, Copy)]
pub struct City {
    pub province: ProvinceId,
    pub is_capital: bool,
}

/// Component that marks a tile as belonging to a province
#[derive(Component, Debug, Clone, Copy)]
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
