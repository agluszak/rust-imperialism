use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use hexx::Hex;
use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::cmp::Reverse;
use crate::tiles::{TileType, TileCategory};

#[derive(Debug, Clone)]
pub struct PathfindingNode {
    pub position: TilePos,
    pub cost: f32,
    pub heuristic: f32,
    pub parent: Option<TilePos>,
}

impl PathfindingNode {
    pub fn total_cost(&self) -> f32 {
        self.cost + self.heuristic
    }
}

impl PartialEq for PathfindingNode {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Eq for PathfindingNode {}

impl PartialOrd for PathfindingNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathfindingNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total_cost().partial_cmp(&other.total_cost()).unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub struct PathfindingSystem;

impl PathfindingSystem {
    pub fn find_path(
        start: TilePos,
        goal: TilePos,
        tilemap_size: &TilemapSize,
        tile_query: &Query<&TileType>,
        tile_storage: &TileStorage,
    ) -> Option<Vec<TilePos>> {
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashMap::new();
        let mut came_from = HashMap::new();
        let mut cost_so_far = HashMap::new();

        let start_node = PathfindingNode {
            position: start,
            cost: 0.0,
            heuristic: Self::heuristic(start, goal),
            parent: None,
        };

        open_set.push(Reverse(start_node));
        cost_so_far.insert(start, 0.0);

        while let Some(Reverse(current)) = open_set.pop() {
            if current.position == goal {
                return Some(Self::reconstruct_path(came_from, current.position));
            }

            if closed_set.contains_key(&current.position) {
                continue;
            }

            closed_set.insert(current.position, current.cost);

            for neighbor in Self::get_neighbors(current.position, tilemap_size) {
                if closed_set.contains_key(&neighbor) {
                    continue;
                }

                let movement_cost = Self::get_movement_cost(neighbor, tile_query, tile_storage);
                if movement_cost < 0.0 {
                    continue; // Impassable tile
                }

                let tentative_cost = current.cost + movement_cost;
                
                if let Some(&existing_cost) = cost_so_far.get(&neighbor) {
                    if tentative_cost >= existing_cost {
                        continue;
                    }
                }

                cost_so_far.insert(neighbor, tentative_cost);
                came_from.insert(neighbor, current.position);

                let neighbor_node = PathfindingNode {
                    position: neighbor,
                    cost: tentative_cost,
                    heuristic: Self::heuristic(neighbor, goal),
                    parent: Some(current.position),
                };

                open_set.push(Reverse(neighbor_node));
            }
        }

        None // No path found
    }

    fn heuristic(from: TilePos, to: TilePos) -> f32 {
        // Hexagonal distance using cube coordinates
        let from_hex = Hex::new(from.x as i32, from.y as i32);
        let to_hex = Hex::new(to.x as i32, to.y as i32);
        from_hex.distance_to(to_hex) as f32
    }

    fn get_neighbors(pos: TilePos, tilemap_size: &TilemapSize) -> Vec<TilePos> {
        let mut neighbors = Vec::new();
        let hex = Hex::new(pos.x as i32, pos.y as i32);
        
        // Get hexagonal neighbors
        for neighbor_hex in hex.all_neighbors() {
            let neighbor_pos = TilePos {
                x: neighbor_hex.x as u32,
                y: neighbor_hex.y as u32,
            };
            
            // Check bounds
            if neighbor_pos.x < tilemap_size.x && neighbor_pos.y < tilemap_size.y {
                neighbors.push(neighbor_pos);
            }
        }
        
        neighbors
    }

    fn get_movement_cost(
        pos: TilePos,
        tile_query: &Query<&TileType>,
        tile_storage: &TileStorage,
    ) -> f32 {
        if let Some(tile_entity) = tile_storage.get(&pos) {
            if let Ok(tile_type) = tile_query.get(tile_entity) {
                if !tile_type.properties.is_passable {
                    return -1.0; // Impassable
                }
                return tile_type.properties.movement_cost;
            }
        }
        1.0 // Default cost
    }

    fn reconstruct_path(came_from: HashMap<TilePos, TilePos>, mut current: TilePos) -> Vec<TilePos> {
        let mut path = vec![current];
        
        while let Some(&parent) = came_from.get(&current) {
            current = parent;
            path.push(current);
        }
        
        path.reverse();
        path
    }

    pub fn calculate_path_cost(
        path: &[TilePos],
        tile_query: &Query<&TileType>,
        tile_storage: &TileStorage,
    ) -> u32 {
        let mut total_cost = 0.0;
        
        for pos in path.iter().skip(1) { // Skip starting position
            total_cost += Self::get_movement_cost(*pos, tile_query, tile_storage);
        }
        
        total_cost.ceil() as u32
    }
}