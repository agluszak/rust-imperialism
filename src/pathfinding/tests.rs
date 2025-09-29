#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::test_with_tilemap;
    use crate::tiles::{TerrainType, TileType};
    use bevy_ecs_tilemap::prelude::*;

    test_with_tilemap!(test_pathfinding_straight_line, 5, 5, {
        let start = TilePos { x: 0, y: 0 };
        let goal = TilePos { x: 2, y: 0 };
        let tilemap_size = TilemapSize { x: 5, y: 5 };

        // Create a simple query for tiles - all grass terrain
        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            start,
            goal,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_some());
        let path = path.unwrap();

        // Path should include start and goal
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));

        // Verify path is valid (adjacent steps)
        assert_valid_path(&path);

        // For a straight line, path length should be reasonable
        assert!(path.len() <= 5); // Should be efficient
    });

    test_with_tilemap!(test_pathfinding_no_path_to_unreachable, 5, 5, {
        let start = TilePos { x: 0, y: 0 };
        let goal = TilePos { x: 10, y: 10 }; // Outside the map
        let tilemap_size = TilemapSize { x: 5, y: 5 };

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            start,
            goal,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_none()); // Should be no path to unreachable position
    });

    test_with_tilemap!(test_pathfinding_same_position, 3, 3, {
        let pos = TilePos { x: 1, y: 1 };
        let tilemap_size = TilemapSize { x: 3, y: 3 };

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            pos,
            pos,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_some());
        let path = path.unwrap();

        // Path to same position should just be the position itself
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], pos);
    });

    test_with_tilemap!(test_pathfinding_with_obstacles, 5, 5, {
        let start = TilePos { x: 0, y: 2 };
        let goal = TilePos { x: 4, y: 2 };
        let tilemap_size = TilemapSize { x: 5, y: 5 };

        // Create water obstacles in the middle
        let obstacle1 = TilePos { x: 2, y: 2 };
        let _obstacle_entity = create_test_tile(
            &mut world,
            obstacle1,
            TerrainType::Water,
            _tilemap_entity,
            &mut _tile_storage,
        );

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            start,
            goal,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_some());
        let path = path.unwrap();

        // Path should not go through water (impassable)
        assert!(!path.contains(&obstacle1));

        // Verify path is valid
        assert_valid_path(&path);

        // Path should start at start and end at goal
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
    });

    test_with_tilemap!(test_path_cost_calculation, 3, 3, {
        // Create tiles with different movement costs
        let pos1 = TilePos { x: 0, y: 0 }; // Grass (cost 1)
        let pos2 = TilePos { x: 1, y: 0 }; // Mountain (cost 3)
        let pos3 = TilePos { x: 2, y: 0 }; // Grass (cost 1)

        let _mountain_entity = create_test_tile(
            &mut world,
            pos2,
            TerrainType::Mountain,
            _tilemap_entity,
            &mut _tile_storage,
        );

        let path = vec![pos1, pos2, pos3];

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let cost = PathfindingSystem::calculate_path_cost_with_combined_query(
            &path,
            &tile_query,
            &_tile_storage,
        );

        // Cost should be 3 + 1 = 4 (skip start position, mountain cost 3, grass cost 1)
        assert_eq!(cost, 4);
    });

    test_with_tilemap!(test_path_cost_empty_path, 3, 3, {
        let empty_path = vec![];
        let tile_query = world.query::<(&TileType, &TilePos)>();

        let cost = PathfindingSystem::calculate_path_cost_with_combined_query(
            &empty_path,
            &tile_query,
            &_tile_storage,
        );

        assert_eq!(cost, 0);
    });

    test_with_tilemap!(test_path_cost_single_position, 3, 3, {
        let single_path = vec![TilePos { x: 1, y: 1 }];
        let tile_query = world.query::<(&TileType, &TilePos)>();

        let cost = PathfindingSystem::calculate_path_cost_with_combined_query(
            &single_path,
            &tile_query,
            &_tile_storage,
        );

        // Single position path has no movement cost
        assert_eq!(cost, 0);
    });

    test_with_tilemap!(test_pathfinding_diagonal_movement, 4, 4, {
        let start = TilePos { x: 0, y: 0 };
        let goal = TilePos { x: 3, y: 3 };
        let tilemap_size = TilemapSize { x: 4, y: 4 };

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            start,
            goal,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_some());
        let path = path.unwrap();

        // Verify path connects start to goal
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));

        // Verify each step is valid (adjacent in hex grid)
        assert_valid_path(&path);

        // Path should be reasonably efficient
        assert!(path.len() <= 8); // Generous upper bound
    });

    test_with_tilemap!(test_pathfinding_around_water_barrier, 7, 5, {
        let start = TilePos { x: 0, y: 2 };
        let goal = TilePos { x: 6, y: 2 };
        let tilemap_size = TilemapSize { x: 7, y: 5 };

        // Create a vertical water barrier
        for y in 0..5 {
            let barrier_pos = TilePos { x: 3, y };
            let _barrier_entity = create_test_tile(
                &mut world,
                barrier_pos,
                TerrainType::Water,
                _tilemap_entity,
                &mut _tile_storage,
            );
        }

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let path = PathfindingSystem::find_path_with_combined_query(
            start,
            goal,
            &tilemap_size,
            &tile_query,
            &_tile_storage,
        );

        assert!(path.is_some());
        let path = path.unwrap();

        // Path should not go through water
        for y in 0..5 {
            assert!(!path.contains(&TilePos { x: 3, y }));
        }

        // Path should reach the goal
        assert_eq!(path.last(), Some(&goal));
        assert_valid_path(&path);
    });

    #[test]
    fn test_pathfinding_system_struct() {
        // Test that PathfindingSystem can be instantiated (if it has any static structure)
        // This mainly ensures the type compiles and is accessible

        // Note: Since PathfindingSystem appears to be a utility struct with static methods,
        // we're mainly testing that it exists and can be referenced
        let _system_name = std::any::type_name::<PathfindingSystem>();
        assert!(_system_name.contains("PathfindingSystem"));
    }

    test_with_tilemap!(test_path_cost_with_mixed_terrain, 4, 1, {
        // Create a path through different terrain types
        let grass_pos = TilePos { x: 1, y: 0 };
        let forest_pos = TilePos { x: 2, y: 0 };
        let mountain_pos = TilePos { x: 3, y: 0 };

        let _forest_entity = create_test_tile(
            &mut world,
            forest_pos,
            TerrainType::Forest,
            _tilemap_entity,
            &mut _tile_storage,
        );
        let _mountain_entity = create_test_tile(
            &mut world,
            mountain_pos,
            TerrainType::Mountain,
            _tilemap_entity,
            &mut _tile_storage,
        );

        let path = vec![
            TilePos { x: 0, y: 0 }, // Start (grass, cost ignored)
            grass_pos,              // Grass (cost 1)
            forest_pos,             // Forest (cost 2)
            mountain_pos,           // Mountain (cost 3)
        ];

        let tile_query = world.query::<(&TileType, &TilePos)>();

        let cost = PathfindingSystem::calculate_path_cost_with_combined_query(
            &path,
            &tile_query,
            &_tile_storage,
        );

        // Expected: 1 (grass) + 2 (forest) + 3 (mountain) = 6
        assert_eq!(cost, 6);
    });
}
