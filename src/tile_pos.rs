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
        // When using HexCoordSystem::Row, TilePos is already in axial coordinates
        // TilePos.x = q, TilePos.y = r
        // No conversion needed - just map directly
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
        // Use bevy_ecs_tilemap's built-in coordinate conversion
        // to ensure we match exactly how tiles are positioned
        use bevy_ecs_tilemap::prelude::*;

        let map_size = TilemapSize {
            x: crate::constants::MAP_SIZE,
            y: crate::constants::MAP_SIZE,
        };

        let tile_size = TilemapTileSize {
            x: TILE_SIZE,
            y: TILE_SIZE,
        };

        let grid_size = crate::constants::get_hex_grid_size();
        let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

        let pos = self.center_in_world(
            &map_size,
            &grid_size,
            &tile_size,
            &map_type,
            &TilemapAnchor::Center,
        );

        bevy::prelude::Vec2::new(pos.x, pos.y)
    }
}

pub trait HexExt {
    fn to_tile_pos(&self) -> Option<TilePos>;
}

impl HexExt for Hex {
    fn to_tile_pos(&self) -> Option<TilePos> {
        // When using HexCoordSystem::Row, TilePos is already in axial coordinates
        // Hex.x = q -> TilePos.x, Hex.y = r -> TilePos.y
        // No conversion needed - just map directly
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_conversion_roundtrip() {
        // Test that converting TilePos -> Hex -> TilePos gives the same result
        let tile = TilePos { x: 5, y: 4 };
        let hex = tile.to_hex();
        let tile_back = hex.to_tile_pos().unwrap();
        assert_eq!(tile, tile_back);
    }

    #[test]
    fn test_hex_neighbors_axial() {
        // Test tile at (5, 4) in axial coordinates (q=5, r=4)
        let tile = TilePos { x: 5, y: 4 };
        let hex = tile.to_hex();

        // Get all neighbors at distance 1
        let neighbors: Vec<_> = hex
            .all_neighbors()
            .iter()
            .filter_map(|h| h.to_tile_pos())
            .collect();

        // In axial coordinates, the 6 neighbors of (q=5, r=4) are:
        // (4,4), (6,4), (4,5), (5,3), (6,3), (5,5)
        let expected = vec![
            TilePos { x: 4, y: 4 },
            TilePos { x: 6, y: 4 },
            TilePos { x: 4, y: 5 },
            TilePos { x: 5, y: 3 },
            TilePos { x: 6, y: 3 },
            TilePos { x: 5, y: 5 },
        ];

        assert_eq!(neighbors.len(), 6, "Should have 6 neighbors");
        for expected_pos in &expected {
            assert!(
                neighbors.contains(expected_pos),
                "Missing neighbor {:?} for tile {:?}",
                expected_pos,
                tile
            );
        }
    }

    #[test]
    fn test_hex_neighbors_another() {
        // Test another tile at (5, 5) in axial coordinates
        let tile = TilePos { x: 5, y: 5 };
        let hex = tile.to_hex();

        // Get all neighbors at distance 1
        let neighbors: Vec<_> = hex
            .all_neighbors()
            .iter()
            .filter_map(|h| h.to_tile_pos())
            .collect();

        // In axial coordinates, the 6 neighbors of (q=5, r=5) are:
        // (4,5), (6,5), (4,6), (5,4), (6,4), (5,6)
        let expected = vec![
            TilePos { x: 4, y: 5 },
            TilePos { x: 6, y: 5 },
            TilePos { x: 4, y: 6 },
            TilePos { x: 5, y: 4 },
            TilePos { x: 6, y: 4 },
            TilePos { x: 5, y: 6 },
        ];

        assert_eq!(neighbors.len(), 6, "Should have 6 neighbors");
        for expected_pos in &expected {
            assert!(
                neighbors.contains(expected_pos),
                "Missing neighbor {:?} for tile {:?}",
                expected_pos,
                tile
            );
        }
    }

    #[test]
    fn test_adjacency_check() {
        // Test that distance_to correctly identifies adjacent tiles in axial coordinates
        let center = TilePos { x: 5, y: 4 }; // axial (q=5, r=4)
        let center_hex = center.to_hex();

        // Adjacent tiles (distance = 1) in axial coordinates
        let adjacent = vec![
            TilePos { x: 4, y: 4 },
            TilePos { x: 6, y: 4 },
            TilePos { x: 4, y: 5 },
            TilePos { x: 5, y: 3 },
            TilePos { x: 6, y: 3 },
            TilePos { x: 5, y: 5 },
        ];

        for adj in &adjacent {
            let adj_hex = adj.to_hex();
            assert_eq!(
                center_hex.distance_to(adj_hex),
                1,
                "Tile {:?} should be adjacent to {:?}",
                adj,
                center
            );
        }

        // Non-adjacent tile (distance > 1)
        let far = TilePos { x: 3, y: 4 };
        let far_hex = far.to_hex();
        assert!(
            center_hex.distance_to(far_hex) > 1,
            "Tile {:?} should not be adjacent to {:?}",
            far,
            center
        );
    }
}
