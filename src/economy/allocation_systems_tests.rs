#[cfg(test)]
mod tests {
    use crate::economy::{
        allocation::Allocations,
        allocation_systems::calculate_inputs_for_one_unit,
        goods::Good,
        production::{Building, BuildingKind, Buildings},
        reservation::ReservationSystem,
        stockpile::Stockpile,
        treasury::Treasury,
        workforce::Workforce,
    };

    /// Test the intelligent input selection logic for Textile Mill
    #[test]
    fn test_textile_mill_prefers_cotton() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 100);
        stockpile.add(Good::Wool, 50);

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], (Good::Cotton, 2));
    }

    #[test]
    fn test_textile_mill_falls_back_to_wool() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 0); // No cotton
        stockpile.add(Good::Wool, 100);

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], (Good::Wool, 2));
    }

    #[test]
    fn test_textile_mill_uses_wool_when_more_available() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Cotton, 1); // Less than needed
        stockpile.add(Good::Wool, 100);

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::TextileMill, Good::Fabric, &stockpile);

        // Should use Wool because Cotton < 2
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], (Good::Wool, 2));
    }

    #[test]
    fn test_food_processing_prefers_fish() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Grain, 100);
        stockpile.add(Good::Fruit, 100);
        stockpile.add(Good::Fish, 50);
        stockpile.add(Good::Livestock, 50);

        let inputs = calculate_inputs_for_one_unit(
            BuildingKind::FoodProcessingCenter,
            Good::CannedFood,
            &stockpile,
        );

        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0], (Good::Grain, 2));
        assert_eq!(inputs[1], (Good::Fruit, 1));
        assert_eq!(inputs[2], (Good::Fish, 1)); // Should prefer Fish
    }

    #[test]
    fn test_food_processing_falls_back_to_livestock() {
        let mut stockpile = Stockpile::default();
        stockpile.add(Good::Grain, 100);
        stockpile.add(Good::Fruit, 100);
        stockpile.add(Good::Fish, 0); // No fish
        stockpile.add(Good::Livestock, 50);

        let inputs = calculate_inputs_for_one_unit(
            BuildingKind::FoodProcessingCenter,
            Good::CannedFood,
            &stockpile,
        );

        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[2], (Good::Livestock, 1)); // Should use Livestock
    }

    #[test]
    fn test_lumber_mill_lumber_output() {
        let stockpile = Stockpile::default();

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::LumberMill, Good::Lumber, &stockpile);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], (Good::Timber, 2));
    }

    #[test]
    fn test_lumber_mill_paper_output() {
        let stockpile = Stockpile::default();

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::LumberMill, Good::Paper, &stockpile);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], (Good::Timber, 2));
    }

    #[test]
    fn test_steel_mill_inputs() {
        let stockpile = Stockpile::default();

        let inputs =
            calculate_inputs_for_one_unit(BuildingKind::SteelMill, Good::Steel, &stockpile);

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], (Good::Iron, 1));
        assert_eq!(inputs[1], (Good::Coal, 1));
    }

    /// Test reservation system integration
    #[test]
    fn test_reservation_system_basics() {
        let mut reservations = ReservationSystem::default();
        let mut stockpile = Stockpile::default();
        let mut workforce = Workforce::new();
        let mut treasury = Treasury::new(1000);

        stockpile.add(Good::Cotton, 10);
        workforce.add_untrained(5);
        workforce.update_labor_pool(); // Update pool after adding workers

        // Reserve 2 Cotton + 1 Labor
        let res_id = reservations.try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        );

        assert!(res_id.is_some(), "Should be able to reserve resources");
        assert_eq!(stockpile.get_available(Good::Cotton), 8);
        assert_eq!(workforce.labor_pool.available(), 4); // Check labor pool, not recalculated value
    }

    #[test]
    fn test_reservation_fails_on_insufficient_resources() {
        let mut reservations = ReservationSystem::default();
        let mut stockpile = Stockpile::default();
        let mut workforce = Workforce::new();
        let mut treasury = Treasury::new(1000);

        stockpile.add(Good::Cotton, 1); // Not enough
        workforce.add_untrained(5);
        workforce.update_labor_pool(); // Update pool after adding workers

        // Try to reserve 2 Cotton (but only 1 available)
        let res_id = reservations.try_reserve(
            vec![(Good::Cotton, 2)],
            1,
            0,
            &mut stockpile,
            &mut workforce,
            &mut treasury,
        );

        assert!(res_id.is_none(), "Should fail to reserve");
        // Verify nothing was reserved (rollback)
        assert_eq!(stockpile.get_available(Good::Cotton), 1);
        assert_eq!(workforce.labor_pool.available(), 5); // Check labor pool, not recalculated value
    }

    #[test]
    fn test_reservation_release() {
        let mut reservations = ReservationSystem::default();
        let mut stockpile = Stockpile::default();
        let mut workforce = Workforce::new();
        let mut treasury = Treasury::new(1000);

        stockpile.add(Good::Cotton, 10);
        workforce.add_untrained(5);
        workforce.update_labor_pool(); // Update pool after adding workers

        // Reserve resources
        let res_id = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();

        assert_eq!(stockpile.get_available(Good::Cotton), 8);
        assert_eq!(workforce.labor_pool.available(), 4); // Check labor pool, not recalculated value

        // Release reservation
        reservations.release(res_id, &mut stockpile, &mut workforce, &mut treasury);

        assert_eq!(stockpile.get_available(Good::Cotton), 10);
        assert_eq!(workforce.labor_pool.available(), 5); // Check labor pool, not recalculated value
    }

    #[test]
    fn test_allocations_tracking() {
        use bevy::prelude::Entity;
        let mut allocations = Allocations::default();
        let mut reservations = ReservationSystem::default();
        let mut stockpile = Stockpile::default();
        let mut workforce = Workforce::new();
        let mut treasury = Treasury::new(1000);

        // Mock entities
        let building = Entity::from_bits(1);

        stockpile.add(Good::Cotton, 20);
        workforce.add_untrained(5);
        workforce.update_labor_pool(); // Update pool after adding workers

        // Add some production allocations using real reservations
        let res1 = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        let res2 = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();

        allocations
            .production
            .entry((building, Good::Fabric))
            .or_default()
            .push(res1);
        allocations
            .production
            .entry((building, Good::Fabric))
            .or_default()
            .push(res2);

        // Check count
        assert_eq!(allocations.production_count(building, Good::Fabric), 2);
        assert_eq!(allocations.production_count(building, Good::Lumber), 0);
    }

    #[test]
    fn test_buildings_collection() {
        let mut buildings = Buildings::new();

        // Add buildings
        buildings.insert(Building::textile_mill(10));
        buildings.insert(Building::lumber_mill(8));

        // Check retrieval
        assert!(buildings.get(BuildingKind::TextileMill).is_some());
        assert!(buildings.get(BuildingKind::LumberMill).is_some());
        assert!(buildings.get(BuildingKind::SteelMill).is_none());

        // Check capacity
        assert_eq!(
            buildings.get(BuildingKind::TextileMill).unwrap().capacity,
            10
        );
    }

    #[test]
    fn test_buildings_with_all_initial() {
        let buildings = Buildings::with_all_initial();

        // Should have all 4 production buildings
        assert!(buildings.get(BuildingKind::TextileMill).is_some());
        assert!(buildings.get(BuildingKind::LumberMill).is_some());
        assert!(buildings.get(BuildingKind::SteelMill).is_some());
        assert!(buildings.get(BuildingKind::FoodProcessingCenter).is_some());

        // Check default capacities
        assert_eq!(
            buildings.get(BuildingKind::TextileMill).unwrap().capacity,
            8
        );
        assert_eq!(buildings.get(BuildingKind::LumberMill).unwrap().capacity, 4);
    }

    #[test]
    fn test_multiple_reservations_for_same_building() {
        use bevy::prelude::Entity;
        let mut reservations = ReservationSystem::default();
        let mut stockpile = Stockpile::default();
        let mut workforce = Workforce::new();
        let mut treasury = Treasury::new(1000);

        stockpile.add(Good::Cotton, 20);
        workforce.add_untrained(10);
        workforce.update_labor_pool(); // Update pool after adding workers

        // Make multiple reservations (simulating multiple units)
        let res1 = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        let res2 = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();
        let res3 = reservations
            .try_reserve(
                vec![(Good::Cotton, 2)],
                1,
                0,
                &mut stockpile,
                &mut workforce,
                &mut treasury,
            )
            .unwrap();

        // Should have reserved 6 Cotton and 3 Labor
        assert_eq!(stockpile.get_available(Good::Cotton), 14);
        assert_eq!(workforce.labor_pool.available(), 7); // Check labor pool, not recalculated value

        // Track in allocations
        let mut allocations = Allocations::default();
        let building = Entity::from_bits(1);
        allocations
            .production
            .entry((building, Good::Fabric))
            .or_default()
            .extend([res1, res2, res3]);

        assert_eq!(allocations.production_count(building, Good::Fabric), 3);
    }
}
