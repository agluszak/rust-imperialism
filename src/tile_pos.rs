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
        // Convert from row-based offset coordinates to axial coordinates
        // For pointy-top hexagons with HexCoordSystem::Row (even-r)
        // TilePos.x = column, TilePos.y = row
        // In even-r: EVEN rows are offset right, ODD rows are not offset
        let col = self.x as i32;
        let row = self.y as i32;

        // Even-r offset to axial conversion:
        // q = col - (row + (row & 1)) / 2
        // r = row
        let q = col - (row + (row & 1)) / 2;
        let r = row;

        Hex::new(q, r)
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

        let grid_size: TilemapGridSize = tile_size.into();
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
        // Convert from axial coordinates to row-based offset coordinates
        // For pointy-top hexagons with HexCoordSystem::Row (even-r)
        // Hex uses (q, r) axial coordinates
        // In even-r: EVEN rows are offset right, ODD rows are not offset
        let q = self.x;
        let r = self.y;

        // Axial to even-r offset conversion:
        // col = q + (r + (r & 1)) / 2
        // row = r
        let col = q + (r + (r & 1)) / 2;
        let row = r;

        if col >= 0 && row >= 0 {
            Some(TilePos {
                x: col as u32,
                y: row as u32,
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
    fn test_hex_neighbors_even_row() {
        // Test tile at (5, 4) - even row
        let tile = TilePos { x: 5, y: 4 };
        let hex = tile.to_hex();

        // Get all neighbors at distance 1
        let neighbors: Vec<_> = hex
            .all_neighbors()
            .iter()
            .filter_map(|h| h.to_tile_pos())
            .collect();

        // For even row (y=4) in even-r system, neighbors should be:
        // Same row: (4,4), (6,4)
        // Upper row: (5,3), (6,3)
        // Lower row: (5,5), (6,5)
        let expected = vec![
            TilePos { x: 4, y: 4 },
            TilePos { x: 6, y: 4 },
            TilePos { x: 5, y: 3 },
            TilePos { x: 6, y: 3 },
            TilePos { x: 5, y: 5 },
            TilePos { x: 6, y: 5 },
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
    fn test_hex_neighbors_odd_row() {
        // Test tile at (5, 5) - odd row
        let tile = TilePos { x: 5, y: 5 };
        let hex = tile.to_hex();

        // Get all neighbors at distance 1
        let neighbors: Vec<_> = hex
            .all_neighbors()
            .iter()
            .filter_map(|h| h.to_tile_pos())
            .collect();

        // For odd row (y=5) in even-r system, neighbors should be:
        // Same row: (4,5), (6,5)
        // Upper row: (4,4), (5,4)
        // Lower row: (4,6), (5,6)
        let expected = vec![
            TilePos { x: 4, y: 5 },
            TilePos { x: 6, y: 5 },
            TilePos { x: 4, y: 4 },
            TilePos { x: 5, y: 4 },
            TilePos { x: 4, y: 6 },
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
        // Test that distance_to correctly identifies adjacent tiles
        let center = TilePos { x: 5, y: 4 }; // even row
        let center_hex = center.to_hex();

        // Adjacent tiles (distance = 1) for even row in even-r
        let adjacent = vec![
            TilePos { x: 4, y: 4 },
            TilePos { x: 6, y: 4 },
            TilePos { x: 5, y: 3 },
            TilePos { x: 6, y: 3 },
            TilePos { x: 5, y: 5 },
            TilePos { x: 6, y: 5 },
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
