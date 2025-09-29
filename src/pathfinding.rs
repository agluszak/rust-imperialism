use crate::constants::*;
use crate::tile_pos::{HexExt, TilePosExt};
use crate::tiles::TileType;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use hexx::{Hex, algorithms::a_star};

pub struct PathfindingSystem;

impl PathfindingSystem {
    /// Find a path using a combined tile query (for use with ParamSet)
    pub fn find_path_with_combined_query(
        start: TilePos,
        goal: TilePos,
        tilemap_size: &TilemapSize,
        tile_query: &Query<(&TileType, &TilePos)>,
        tile_storage: &TileStorage,
    ) -> Option<Vec<TilePos>> {
        let start_hex = start.to_hex();
        let goal_hex = goal.to_hex();

        // Create a cost function that considers tile movement costs and bounds
        let cost_fn = |_from: Hex, to: Hex| -> Option<u32> {
            // Convert hex back to tile position for bounds checking
            let to_pos = to.to_tile_pos()?;

            // Check bounds
            if to_pos.x >= tilemap_size.x || to_pos.y >= tilemap_size.y {
                return None;
            }

            // Get tile entity and check passability/cost
            if let Some(tile_entity) = tile_storage.get(&to_pos)
                && let Ok((tile_type, _)) = tile_query.get(tile_entity)
            {
                if !tile_type.properties.is_passable {
                    return None; // Impassable tile
                }
                return Some(tile_type.properties.movement_cost.ceil() as u32);
            }

            Some(1) // Default movement cost
        };

        // Use hexx's A* algorithm
        let hex_path = a_star(start_hex, goal_hex, cost_fn)?;

        // Convert hex path back to tile positions
        hex_path
            .into_iter()
            .filter_map(|hex| hex.to_tile_pos())
            .collect::<Vec<_>>()
            .into()
    }

    /// Calculate the total movement cost for a path using combined query
    pub fn calculate_path_cost_with_combined_query(
        path: &[TilePos],
        tile_query: &Query<(&TileType, &TilePos)>,
        tile_storage: &TileStorage,
    ) -> u32 {
        let mut total_cost = 0.0;

        for pos in path.iter().skip(1) {
            // Skip starting position
            if let Some(tile_entity) = tile_storage.get(pos)
                && let Ok((tile_type, _)) = tile_query.get(tile_entity)
            {
                if !tile_type.properties.is_passable {
                    total_cost += IMPASSABLE_TILE_COST;
                } else {
                    total_cost += tile_type.properties.movement_cost;
                }
            } else {
                total_cost += 1.0;
            }
        }

        total_cost.ceil() as u32
    }
}

#[cfg(test)]
mod tests;
