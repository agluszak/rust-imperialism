#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_creation() {
        let health = Health::new(100);
        assert_eq!(health.current, 100);
        assert_eq!(health.max, 100);
        assert!(health.is_alive());
        assert!(!health.is_low_health());
    }

    #[test]
    fn test_health_damage() {
        let mut health = Health::new(50);

        health.take_damage(20);
        assert_eq!(health.current, 30);
        assert!(health.is_alive());

        health.take_damage(25);
        assert_eq!(health.current, 5);
        assert!(health.is_alive());
        assert!(health.is_low_health());

        health.take_damage(10);
        assert_eq!(health.current, 0);
        assert!(!health.is_alive());
    }

    #[test]
    fn test_health_healing() {
        let mut health = Health::new(100);
        health.take_damage(60);
        assert_eq!(health.current, 40);

        health.heal(20);
        assert_eq!(health.current, 60);

        // Cannot heal above max
        health.heal(50);
        assert_eq!(health.current, 100);
        assert_eq!(health.max, 100);
    }

    #[test]
    fn test_health_full_heal() {
        let mut health = Health::new(80);
        health.take_damage(70);
        assert_eq!(health.current, 10);
        assert!(health.is_low_health());

        health.heal_to_full();
        assert_eq!(health.current, 80);
        assert_eq!(health.max, 80);
        assert!(!health.is_low_health());
    }

    #[test]
    fn test_health_percentage() {
        let mut health = Health::new(100);

        assert_eq!(health.percentage(), 100);

        health.take_damage(25);
        assert_eq!(health.percentage(), 75);

        health.take_damage(50);
        assert_eq!(health.percentage(), 25);

        health.take_damage(25);
        assert_eq!(health.percentage(), 0);
    }

    #[test]
    fn test_health_low_threshold() {
        let mut health = Health::new(100);

        // Not low health at 30% (threshold is 25%)
        health.take_damage(70);
        assert_eq!(health.current, 30);
        assert!(!health.is_low_health());

        // Low health at 25%
        health.take_damage(5);
        assert_eq!(health.current, 25);
        assert!(health.is_low_health());

        // Still low health at lower percentages
        health.take_damage(20);
        assert_eq!(health.current, 5);
        assert!(health.is_low_health());
    }

    #[test]
    fn test_health_edge_cases() {
        // Zero max health
        let health = Health::new(0);
        assert!(!health.is_alive());
        assert_eq!(health.percentage(), 0);

        // Single point of health
        let mut health = Health::new(1);
        assert!(health.is_alive());
        assert!(health.is_low_health()); // 1 health is always low

        health.take_damage(1);
        assert!(!health.is_alive());
        assert_eq!(health.current, 0);
    }

    #[test]
    fn test_combat_creation() {
        let combat = Combat::new(25);
        assert_eq!(combat.attack_damage, 25);
    }

    #[test]
    fn test_combat_default() {
        let combat = Combat::default();
        assert_eq!(combat.attack_damage, 10);
    }

    #[test]
    fn test_combat_damage_values() {
        let weak_combat = Combat::new(5);
        let strong_combat = Combat::new(100);

        assert_eq!(weak_combat.attack_damage, 5);
        assert_eq!(strong_combat.attack_damage, 100);
    }

    #[test]
    fn test_health_overdamage() {
        let mut health = Health::new(10);

        // Damage more than current health
        health.take_damage(20);
        assert_eq!(health.current, 0);
        assert!(!health.is_alive());
    }

    #[test]
    fn test_health_overheal() {
        let mut health = Health::new(50);
        health.take_damage(20);
        assert_eq!(health.current, 30);

        // Heal more than max
        health.heal(100);
        assert_eq!(health.current, 50); // Should cap at max
        assert_eq!(health.max, 50);
    }

    #[test]
    fn test_health_zero_damage() {
        let mut health = Health::new(50);
        health.take_damage(0);
        assert_eq!(health.current, 50); // No change
    }

    #[test]
    fn test_health_zero_heal() {
        let mut health = Health::new(50);
        health.take_damage(10);
        health.heal(0);
        assert_eq!(health.current, 40); // No change
    }

    #[test]
    fn test_health_boundary_conditions() {
        let mut health = Health::new(4);

        // Test exact 25% threshold
        health.take_damage(3);
        assert_eq!(health.current, 1);
        assert_eq!(health.percentage(), 25);
        assert!(health.is_low_health());

        // Test just above 25% threshold
        let mut health = Health::new(4);
        health.take_damage(2);
        assert_eq!(health.current, 2);
        assert_eq!(health.percentage(), 50);
        assert!(!health.is_low_health());
    }
}
