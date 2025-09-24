use bevy_ecs_tilemap::prelude::TilePos;
use hexx::Hex;

pub trait TilePosExt {
    fn to_hex(&self) -> Hex;
}

impl TilePosExt for TilePos {
    fn to_hex(&self) -> Hex {
        Hex::new(self.x as i32, self.y as i32)
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
