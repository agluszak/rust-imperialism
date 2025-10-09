#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    use super::super::{
        allocation::*,
        allocation_systems::*,
        goods::Good,
        production::{Building, BuildingKind, ProductionChoice},
        stockpile::Stockpile,
        treasury::Treasury,
        workforce::{RecruitmentCapacity, types::*},
    };
    use crate::province::{Province, ProvinceId};
    use bevy_ecs_tilemap::prelude::TilePos;

    /// Helper to create a test world with allocation systems registered
    fn create_test_world() -> World {
        let mut world = World::new();

        // Register messages
        world.init_resource::<Events<AdjustRecruitment>>();
        world.init_resource::<Events<AdjustTraining>>();
        world.init_resource::<Events<AdjustProduction>>();

        world
    }

    #[test]
    fn test_production_allocation_caps_by_inputs() {
        let mut world = create_test_world();

        // Setup: Nation with only 6 cotton (enough for 3 fabric, since ratio is 2:1)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 6);

        let mut workforce = Workforce::default();
        for _ in 0..20 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let nation = world
            .spawn((ResourceAllocations::default(), stockpile, workforce))
            .id();

        // Building with capacity 10 (higher than input constraint)
        let building = world
            .spawn(Building {
                kind: BuildingKind::TextileMill,
                capacity: 10,
            })
            .id();

        // Send event 5 times to try to allocate 5 fabric
        for _ in 0..5 {
            world.write_message(AdjustProduction {
                nation,
                building,
                output_good: Good::Fabric,
                choice: Some(ProductionChoice::UseCotton),
                target_output: 5,
            });
        }

        // Run the system
        world.run_system_once(apply_production_adjustments);

        // Check: Should be capped to 3 (6 cotton / 2 = 3 fabric)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let prod_alloc = allocations.production.get(&building).unwrap();
        let fabric_output = prod_alloc
            .outputs
            .get(&Good::Fabric)
            .map(|o| o.allocated)
            .unwrap_or(0);

        assert_eq!(
            fabric_output, 3,
            "Should be capped by cotton availability (6 cotton = 3 fabric)"
        );
    }

    #[test]
    fn test_production_allocation_caps_by_capacity() {
        let mut world = create_test_world();

        // Setup: Nation with plenty of resources
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 100);

        let mut workforce = Workforce::default();
        for _ in 0..20 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let nation = world
            .spawn((ResourceAllocations::default(), stockpile, workforce))
            .id();

        // Building with capacity 5
        let building = world
            .spawn(Building {
                kind: BuildingKind::TextileMill,
                capacity: 5,
            })
            .id();

        // Send event trying to allocate 10 fabric
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Fabric,
            choice: Some(ProductionChoice::UseCotton),
            target_output: 10,
        });

        world.run_system_once(apply_production_adjustments);

        // Check: Should be capped to 5 (capacity)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let prod_alloc = allocations.production.get(&building).unwrap();
        let fabric_output = prod_alloc
            .outputs
            .get(&Good::Fabric)
            .map(|o| o.allocated)
            .unwrap_or(0);

        assert_eq!(fabric_output, 5, "Should be capped by building capacity");
    }

    #[test]
    fn test_recruitment_allocation_caps_by_resources() {
        let mut world = create_test_world();

        // Setup: Nation with only 3 furniture (limiting resource)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::CannedFood, 10);
        stockpile.add(Good::Clothing, 10);
        stockpile.add(Good::Furniture, 3); // Limiting factor

        let nation = world
            .spawn((
                ResourceAllocations::default(),
                stockpile,
                RecruitmentCapacity { upgraded: false },
            ))
            .id();

        // Spawn 20 provinces (capacity = 20/4 = 5)
        for _ in 0..20 {
            world.spawn(Province {
                id: ProvinceId(0),
                tiles: vec![TilePos { x: 0, y: 0 }],
                city_tile: TilePos { x: 0, y: 0 },
                owner: Some(nation),
            });
        }

        // Send event 5 times trying to allocate 5 workers
        for _ in 0..5 {
            world.write_message(AdjustRecruitment {
                nation,
                requested: 5,
            });
        }

        world.run_system_once(apply_recruitment_adjustments);

        // Check: Should be capped to 3 (furniture is limiting)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();

        assert_eq!(
            allocations.recruitment.requested, 3,
            "Should be capped by furniture (3)"
        );
        assert_eq!(allocations.recruitment.allocated, 3);
    }

    #[test]
    fn test_recruitment_allocation_caps_by_capacity() {
        let mut world = create_test_world();

        // Setup: Nation with plenty of resources
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::CannedFood, 100);
        stockpile.add(Good::Clothing, 100);
        stockpile.add(Good::Furniture, 100);

        let nation = world
            .spawn((
                ResourceAllocations::default(),
                stockpile,
                RecruitmentCapacity { upgraded: false },
            ))
            .id();

        // Spawn 8 provinces (capacity = 8/4 = 2 with upgraded=false)
        for _ in 0..8 {
            world.spawn(Province {
                id: ProvinceId(0),
                tiles: vec![TilePos { x: 0, y: 0 }],
                city_tile: TilePos { x: 0, y: 0 },
                owner: Some(nation),
            });
        }

        // Try to allocate 10 workers
        world.write_message(AdjustRecruitment {
            nation,
            requested: 10,
        });

        world.run_system_once(apply_recruitment_adjustments);

        // Check: Should be capped to 2 (capacity = 8 provinces / 4)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();

        assert_eq!(
            allocations.recruitment.requested, 2,
            "Should be capped by recruitment capacity (8/4 = 2)"
        );
        assert_eq!(allocations.recruitment.allocated, 2);
    }

    #[test]
    fn test_training_allocation_caps_by_workers() {
        let mut world = create_test_world();

        // Setup: Nation with only 3 untrained workers
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Paper, 100); // Plenty of paper

        let mut workforce = Workforce::default();
        for _ in 0..3 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Untrained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let treasury = Treasury(10000); // Plenty of cash

        let nation = world
            .spawn((
                ResourceAllocations::default(),
                stockpile,
                workforce,
                treasury,
            ))
            .id();

        // Send event 5 times trying to train 5 workers
        for _ in 0..5 {
            world.write_message(AdjustTraining {
                nation,
                from_skill: WorkerSkill::Untrained,
                requested: 5,
            });
        }

        world.run_system_once(apply_training_adjustments);

        // Check: Should be capped to 3 (only 3 untrained workers)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let training_alloc = allocations
            .training
            .iter()
            .find(|t| t.from_skill == WorkerSkill::Untrained)
            .unwrap();

        assert_eq!(
            training_alloc.requested, 3,
            "Should be capped by available workers (3)"
        );
        assert_eq!(training_alloc.allocated, 3);
    }

    #[test]
    fn test_training_allocation_caps_by_cash() {
        let mut world = create_test_world();

        // Setup: Nation with only $250 (enough for 2 trainings at $100 each)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Paper, 100);

        let mut workforce = Workforce::default();
        for _ in 0..10 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Untrained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let treasury = Treasury(250); // Only $250 = 2 trainings

        let nation = world
            .spawn((
                ResourceAllocations::default(),
                stockpile,
                workforce,
                treasury,
            ))
            .id();

        // Try to train 5 workers
        world.write_message(AdjustTraining {
            nation,
            from_skill: WorkerSkill::Untrained,
            requested: 5,
        });

        world.run_system_once(apply_training_adjustments);

        // Check: Should be capped to 2 ($250 / $100 = 2)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let training_alloc = allocations
            .training
            .iter()
            .find(|t| t.from_skill == WorkerSkill::Untrained)
            .unwrap();

        assert_eq!(
            training_alloc.requested, 2,
            "Should be capped by cash ($250 / $100 = 2)"
        );
        assert_eq!(training_alloc.allocated, 2);
    }

    #[test]
    fn test_ignores_allocation_when_already_at_max() {
        let mut world = create_test_world();

        // Setup with 3 cotton (max 1 fabric with 2:1 ratio, but need 2 cotton minimum)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 3);

        let workforce = Workforce::default();

        let allocations = ResourceAllocations::default();

        let nation = world.spawn((allocations, stockpile, workforce)).id();

        let building = world
            .spawn(Building {
                kind: BuildingKind::TextileMill,
                capacity: 10,
            })
            .id();

        // First allocation: set to 1
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Fabric,
            choice: Some(ProductionChoice::UseCotton),
            target_output: 1,
        });

        world.run_system_once(apply_production_adjustments);

        let alloc_before = world.get::<ResourceAllocations>(nation).unwrap().clone();

        // Try to increase to 2 (should be ignored, capped at 1)
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Fabric,
            choice: Some(ProductionChoice::UseCotton),
            target_output: 2,
        });

        world.run_system_once(apply_production_adjustments);

        let alloc_after = world.get::<ResourceAllocations>(nation).unwrap();

        // Should still be 1 (unchanged)
        let fabric_before = alloc_before
            .production
            .get(&building)
            .and_then(|p| p.outputs.get(&Good::Fabric))
            .map(|o| o.allocated)
            .unwrap_or(0);
        let fabric_after = alloc_after
            .production
            .get(&building)
            .and_then(|p| p.outputs.get(&Good::Fabric))
            .map(|o| o.allocated)
            .unwrap_or(0);

        assert_eq!(
            fabric_before, fabric_after,
            "Allocation should remain at 1 (already at max)"
        );
    }

    #[test]
    fn test_ignores_decrement_below_zero() {
        let mut world = create_test_world();

        let stockpile = Stockpile::default(); // Empty
        let allocations = ResourceAllocations::default(); // Already at 0

        let nation = world
            .spawn((
                allocations,
                stockpile,
                RecruitmentCapacity { upgraded: false },
            ))
            .id();

        // Spawn some provinces
        for _ in 0..8 {
            world.spawn(Province {
                id: ProvinceId(0),
                tiles: vec![TilePos { x: 0, y: 0 }],
                city_tile: TilePos { x: 0, y: 0 },
                owner: Some(nation),
            });
        }

        // Try to set to 0 multiple times (simulating decrement when already 0)
        for _ in 0..3 {
            world.write_message(AdjustRecruitment {
                nation,
                requested: 0,
            });
        }

        world.run_system_once(apply_recruitment_adjustments);

        let allocations = world.get::<ResourceAllocations>(nation).unwrap();

        assert_eq!(allocations.recruitment.requested, 0, "Should stay at 0");
        assert_eq!(allocations.recruitment.allocated, 0);
    }

    #[test]
    fn test_steel_mill_needs_both_iron_and_coal() {
        let mut world = create_test_world();

        // Setup: 5 iron, 3 coal (coal is limiting)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Iron, 5);
        stockpile.add(Good::Coal, 3);

        let mut workforce = Workforce::default();
        for _ in 0..10 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let nation = world
            .spawn((ResourceAllocations::default(), stockpile, workforce))
            .id();

        let building = world
            .spawn(Building {
                kind: BuildingKind::SteelMill,
                capacity: 10,
            })
            .id();

        // Try to produce 5 steel (needs 5 iron + 5 coal)
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Steel,
            choice: None, // Steel mill has no choice
            target_output: 5,
        });

        world.run_system_once(apply_production_adjustments);

        // Check: Should be capped to 3 (limited by coal)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let prod_alloc = allocations.production.get(&building).unwrap();
        let steel_output = prod_alloc
            .outputs
            .get(&Good::Steel)
            .map(|o| o.allocated)
            .unwrap_or(0);

        assert_eq!(steel_output, 3, "Should be capped by coal (3)");
    }

    #[test]
    fn test_textile_mill_uses_sum_of_cotton_and_wool() {
        let mut world = create_test_world();

        // Setup: 4 cotton + 6 wool = 10 total fiber (can produce 5 fabric at 2:1 ratio)
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 4);
        stockpile.add(Good::Wool, 6);

        let mut workforce = Workforce::default();
        for _ in 0..10 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let nation = world
            .spawn((ResourceAllocations::default(), stockpile, workforce))
            .id();

        let building = world
            .spawn(Building {
                kind: BuildingKind::TextileMill,
                capacity: 10,
            })
            .id();

        // Try to produce 7 fabric (would need 14 fiber, but only have 10)
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Fabric,
            choice: None, // Should use SUM of cotton and wool
            target_output: 7,
        });

        world.run_system_once(apply_production_adjustments);

        // Check: Should be capped to 5 (10 total fiber / 2 = 5 fabric)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let prod_alloc = allocations.production.get(&building).unwrap();
        let fabric_output = prod_alloc
            .outputs
            .get(&Good::Fabric)
            .map(|o| o.allocated)
            .unwrap_or(0);

        assert_eq!(
            fabric_output, 5,
            "Should be capped by total fiber (4 cotton + 6 wool = 10 total, / 2 = 5 fabric)"
        );
    }

    #[test]
    fn test_lumber_mill_paper_blocks_lumber_allocation() {
        let mut world = create_test_world();

        // Setup: Only 2 timber total
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Timber, 2);

        let mut workforce = Workforce::default();
        for _ in 0..10 {
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
        }

        let nation = world
            .spawn((ResourceAllocations::default(), stockpile, workforce))
            .id();

        let building = world
            .spawn(Building {
                kind: BuildingKind::LumberMill,
                capacity: 10,
            })
            .id();

        // First: Allocate all 2 timber to produce 1 paper
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Paper,
            choice: Some(ProductionChoice::MakePaper),
            target_output: 1, // Uses 2 timber
        });

        world.run_system_once(apply_production_adjustments);

        // Now try to allocate for lumber (should have 0 timber left)
        world.write_message(AdjustProduction {
            nation,
            building,
            output_good: Good::Lumber,
            choice: Some(ProductionChoice::MakeLumber),
            target_output: 1, // Would need 2 timber, but 0 left
        });

        world.run_system_once(apply_production_adjustments);

        // Check: Lumber allocation should be 0 (no timber left)
        let allocations = world.get::<ResourceAllocations>(nation).unwrap();
        let prod_alloc = allocations.production.get(&building).unwrap();

        let lumber_output = prod_alloc
            .outputs
            .get(&Good::Lumber)
            .map(|o| o.allocated)
            .unwrap_or(0);

        // After allocating all timber for paper, lumber cannot be allocated
        assert_eq!(
            lumber_output, 0,
            "After allocating 2 timber for paper, no timber left for lumber"
        );
    }
}
