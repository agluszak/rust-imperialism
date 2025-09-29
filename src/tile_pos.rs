use crate::constants::TILE_SIZE;
use bevy_ecs_tilemap::prelude::*;
use hexx::Hex;

pub trait TilePosExt {
    fn to_hex(&self) -> Hex;

    /// Convert tile position to world position with standard tile size
    fn to_world_pos_standard(
        &self,
        tilemap_size: &TilemapSize,
        grid_size: &TilemapGridSize,
        map_type: &TilemapType,
        z: f32,
    ) -> bevy::prelude::Vec3;
}

impl TilePosExt for TilePos {
    fn to_hex(&self) -> Hex {
        Hex::new(self.x as i32, self.y as i32)
    }

    fn to_world_pos_standard(
        &self,
        tilemap_size: &TilemapSize,
        grid_size: &TilemapGridSize,
        map_type: &TilemapType,
        z: f32,
    ) -> bevy::prelude::Vec3 {
        self.center_in_world(
            tilemap_size,
            grid_size,
            &TilemapTileSize {
                x: TILE_SIZE,
                y: TILE_SIZE,
            },
            map_type,
            &TilemapAnchor::Center,
        )
        .extend(z)
    }
}

pub trait HexExt {
    fn to_tile_pos(&self) -> Option<TilePos>;
}

impl HexExt for Hex {
    fn to_tile_pos(&self) -> Option<TilePos> {
        if self.x >= 0 && self.y >= 0 {
            Some(TilePos {
                x: self.x as u32,
                y: self.y as u32,
            })
        } else {
            None
        }
    }
}
