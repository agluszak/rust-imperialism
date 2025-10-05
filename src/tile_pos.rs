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

    /// Simple conversion to world position using hex layout
    /// Uses a fixed hex layout for the current map setup
    fn to_world_pos(&self) -> bevy::prelude::Vec2;
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

    fn to_world_pos(&self) -> bevy::prelude::Vec2 {
        // Simple conversion that approximates the tilemap's centered positioning
        // The tilemap uses TilemapAnchor::Center, so we need to offset by half the map size
        // Note: This is an approximation for rendering purposes

        // For now, just use a simple centered hex grid calculation
        // Offset tile coordinates by half map size to center at (0, 0)
        let map_half = crate::constants::MAP_SIZE as f32 / 2.0;
        let centered_x = self.x as f32 - map_half;
        let centered_y = self.y as f32 - map_half;

        // Hex flat-top spacing (row-based)
        let hex_spacing = TILE_SIZE;
        let x = centered_x * hex_spacing * 0.75;
        let y = centered_y * hex_spacing + (self.x % 2) as f32 * hex_spacing * 0.5;

        bevy::prelude::Vec2::new(x, y)
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
