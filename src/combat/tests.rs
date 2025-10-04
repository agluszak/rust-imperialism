use super::*;
use crate::health::{Combat, Health};
use crate::hero::Hero;
use crate::monster::{Monster, MonsterBehavior};
use crate::test_utils::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

#[test]
fn test_combat_event_creation() {
    let attacker = Entity::from_bits(1);
    let defender = Entity::from_bits(2);
    let damage = 25;

    let event = CombatEvent {
        attacker: attacker,
        defender,
        damage,
    };

    assert_eq!(event.defender, defender);
    assert_eq!(event.damage, damage);
}

#[test]
fn test_hero_attack_clicked_event() {
    let target_pos = TilePos { x: 5, y: 3 };
    let event = HeroAttackClicked { target_pos };

    assert_eq!(event.target_pos, target_pos);
}

#[test]
fn test_death_event_creation() {
    let entity = Entity::from_bits(42);
    let event = DeathEvent {
        entity,
        was_monster: true,
    };

    assert_eq!(event.entity, entity);
    assert!(event.was_monster);
}

#[test]
fn test_combat_damage_application() {
    let mut world = create_test_world();
    // Set up entities
    let hero_pos = TilePos { x: 1, y: 1 };
    let monster_pos = TilePos { x: 2, y: 1 };

    let hero = world
        .spawn((Hero::default(), Health::new(100), Combat::new(30), hero_pos))
        .id();

    let monster = world
        .spawn((
            Monster {
                name: "Test Monster".to_string(),
                sight_range: 5,
                behavior: MonsterBehavior::Aggressive,
            },
            Health::new(50),
            Combat::new(15),
            monster_pos,
        ))
        .id();

    // Test damage to hero
    {
        let mut health_query = world.query::<&mut Health>();
        let mut monster_health = health_query.get_mut(&mut world, monster).unwrap();

        assert_eq!(monster_health.current, 50);
        monster_health.take_damage(30);
        assert_eq!(monster_health.current, 20);
        assert!(monster_health.is_alive());
    }

    // Test fatal damage
    {
        let mut health_query = world.query::<&mut Health>();
        let mut monster_health = health_query.get_mut(&mut world, monster).unwrap();

        monster_health.take_damage(25);
        assert_eq!(monster_health.current, 0);
        assert!(!monster_health.is_alive());
    }
}

#[test]
fn test_hero_vs_monster_combat_scenario() {
    let mut world = create_test_world();
    let (hero, monster) =
        setup_combat_scenario(&mut world, TilePos { x: 1, y: 1 }, TilePos { x: 2, y: 1 });

    // Verify setup
    let hero_health = world.entity(hero).get::<Health>().unwrap();
    let monster_health = world.entity(monster).get::<Health>().unwrap();

    assert_eq!(hero_health.current, 100);
    assert_eq!(monster_health.current, 50);

    // Simulate combat - hero attacks monster
    let hero_combat = world.entity(hero).get::<Combat>().unwrap().clone();
    let mut monster_health_mut = world.entity_mut(monster);
    let mut monster_health = monster_health_mut.get_mut::<Health>().unwrap();

    let initial_monster_hp = monster_health.current;
    monster_health.take_damage(hero_combat.attack_damage);

    assert_eq!(
        monster_health.current,
        initial_monster_hp.saturating_sub(hero_combat.attack_damage)
    );
}

#[test]
fn test_adjacent_position_check() {
    let mut world = create_test_world();
    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 2, y: 1 }; // Adjacent in hex grid

    // This should not panic - positions are adjacent
    assert_adjacent(pos1, pos2);
}

#[test]
#[should_panic(expected = "not adjacent")]
fn test_non_adjacent_position_check() {
    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 5, y: 5 }; // Not adjacent

    assert_adjacent(pos1, pos2); // Should panic
}

#[test]
fn test_combat_with_different_damage_values() {
    let mut world = create_test_world();
    let attacker = world
        .spawn((
            Combat::new(0), // No damage
            Health::new(100),
        ))
        .id();

    let defender = world.spawn((Health::new(50),)).id();

    // Test zero damage
    {
        let mut health_query = world.query::<&mut Health>();
        let mut defender_health = health_query.get_mut(&mut world, defender).unwrap();

        defender_health.take_damage(0);
        assert_eq!(defender_health.current, 50); // No change
    }

    // Test massive damage
    {
        let mut health_query = world.query::<&mut Health>();
        let mut defender_health = health_query.get_mut(&mut world, defender).unwrap();

        defender_health.take_damage(1000);
        assert_eq!(defender_health.current, 0); // Capped at 0
        assert!(!defender_health.is_alive());
    }
}

#[test]
fn test_combat_state_persistence() {
    let mut world = create_test_world();
    // Test that combat state is maintained across multiple attacks
    let (hero, monster) =
        setup_combat_scenario(&mut world, TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 0 });

    // First attack
    {
        let mut health_query = world.query::<&mut Health>();
        let mut monster_health = health_query.get_mut(&mut world, monster).unwrap();

        monster_health.take_damage(10);
        assert_eq!(monster_health.current, 40);
    }

    // Second attack on same entity
    {
        let mut health_query = world.query::<&mut Health>();
        let mut monster_health = health_query.get_mut(&mut world, monster).unwrap();

        monster_health.take_damage(15);
        assert_eq!(monster_health.current, 25);
    }

    // Third attack - should finish it
    {
        let mut health_query = world.query::<&mut Health>();
        let mut monster_health = health_query.get_mut(&mut world, monster).unwrap();

        monster_health.take_damage(25);
        assert_eq!(monster_health.current, 0);
        assert!(!monster_health.is_alive());
    }
}

#[test]
fn test_hero_kill_tracking() {
    let mut world = create_test_world();
    let mut hero = Hero::default();
    assert_eq!(hero.kills, 0);
    assert!(!hero.should_heal_from_kills());

    // Add some kills
    hero.add_kill();
    assert_eq!(hero.kills, 1);
    assert!(!hero.should_heal_from_kills());

    hero.add_kill();
    assert_eq!(hero.kills, 2);
    assert!(!hero.should_heal_from_kills());

    // Third kill should trigger healing
    hero.add_kill();
    assert_eq!(hero.kills, 3);
    assert!(hero.should_heal_from_kills());

    // Test multiple of 3
    for _ in 0..3 {
        hero.add_kill();
    }
    assert_eq!(hero.kills, 6);
    assert!(hero.should_heal_from_kills());
}

#[test]
fn test_mock_event_writer() {
    let mut world = create_test_world();
    let mut mock_writer = MockEventWriter::<CombatEvent>::new();
    assert!(mock_writer.is_empty());
    assert_eq!(mock_writer.len(), 0);

    let event = CombatEvent {
        attacker: Entity::from_bits(1),
        defender: Entity::from_bits(2),
        damage: 25,
    };

    mock_writer.write(event);
    assert_eq!(mock_writer.len(), 1);
    assert!(!mock_writer.is_empty());

    mock_writer.clear();
    assert!(mock_writer.is_empty());
}

#[test]
fn test_combat_event_fields() {
    let attacker = Entity::from_bits(100);
    let defender = Entity::from_bits(200);
    let damage = 42;

    let event = CombatEvent {
        attacker: attacker,
        defender,
        damage,
    };

    // Test that we can access all fields
    assert_eq!(event.attacker, attacker);
    assert_eq!(event.defender, defender);
    assert_eq!(event.damage, damage);
}

#[test]
fn test_multiple_entities_combat() {
    let mut world = create_test_world();
    // Set up multiple monsters for group combat scenarios
    let hero = create_test_hero(&mut world, TilePos { x: 2, y: 2 });

    let monster1 = create_test_monster(
        &mut world,
        TilePos { x: 1, y: 2 },
        MonsterBehavior::Aggressive,
        30,
    );
    let monster2 = create_test_monster(
        &mut world,
        TilePos { x: 3, y: 2 },
        MonsterBehavior::Aggressive,
        40,
    );
    let monster3 = create_test_monster(
        &mut world,
        TilePos { x: 2, y: 1 },
        MonsterBehavior::Aggressive,
        20,
    );

    // Verify all entities exist and have expected health
    let mut health_query = world.query::<&Health>();

    let hero_health = health_query.get(&world, hero).unwrap();
    assert_eq!(hero_health.current, 100);

    let m1_health = health_query.get(&world, monster1).unwrap();
    assert_eq!(m1_health.current, 30);

    let m2_health = health_query.get(&world, monster2).unwrap();
    assert_eq!(m2_health.current, 40);

    let m3_health = health_query.get(&world, monster3).unwrap();
    assert_eq!(m3_health.current, 20);
}
