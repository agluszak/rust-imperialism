//! Integration tests for rust-imperialism game systems
//!
//! These tests demonstrate ECS testing patterns and verify core game mechanics

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rust_imperialism::*;

/// Basic test that ECS components can be created and used
#[test]
fn test_health_component() {
    use rust_imperialism::health::Health;

    let mut health = Health::new(100);
    assert_eq!(health.current, 100);
    assert_eq!(health.max, 100);
    assert!(health.is_alive());

    health.take_damage(30);
    assert_eq!(health.current, 70);
    assert!(health.is_alive());

    health.heal(20);
    assert_eq!(health.current, 90);
}

/// Test movement action points system
#[test]
fn test_action_points() {
    use rust_imperialism::movement::ActionPoints;

    let mut ap = ActionPoints::new(6);
    assert_eq!(ap.current, 6);
    assert!(!ap.is_exhausted());

    assert!(ap.can_move(3));
    ap.consume(3);
    assert_eq!(ap.current, 3);

    assert!(ap.can_move(3));
    ap.consume(3);
    assert_eq!(ap.current, 0);
    assert!(ap.is_exhausted());

    ap.refresh();
    assert_eq!(ap.current, 6);
}

/// Test hero component functionality
#[test]
fn test_hero_component() {
    use rust_imperialism::hero::Hero;

    let mut hero = Hero::default();
    assert!(!hero.is_selected);
    assert_eq!(hero.kills, 0);

    hero.select();
    assert!(hero.is_selected);

    hero.add_kill();
    assert_eq!(hero.kills, 1);
    assert!(!hero.should_heal_from_kills()); // Needs 3 kills

    hero.add_kill();
    hero.add_kill();
    assert_eq!(hero.kills, 3);
    assert!(hero.should_heal_from_kills()); // Multiple of 3
}

/// Test turn system functionality
#[test]
fn test_turn_system() {
    use rust_imperialism::turn_system::{TurnPhase, TurnSystem};

    let mut turn_system = TurnSystem::default();
    assert_eq!(turn_system.current_turn, 1);
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::Processing);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::EnemyTurn);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);
    assert_eq!(turn_system.current_turn, 2); // New turn
}

/// Test tile system properties
#[test]
fn test_tile_properties() {
    use rust_imperialism::tiles::{TerrainType, TileType};

    let grass = TileType::terrain(TerrainType::Grass);
    assert_eq!(grass.properties.movement_cost, 1.0);
    assert!(grass.properties.is_passable);

    let water = TileType::terrain(TerrainType::Water);
    assert_eq!(water.properties.movement_cost, 2.0);
    assert!(!water.properties.is_passable); // Water is impassable without ships

    let mountain = TileType::terrain(TerrainType::Mountain);
    assert_eq!(mountain.properties.movement_cost, 3.0);
    assert_eq!(mountain.properties.defense_bonus, 2.0);
}

/// Test UI state management
#[test]
fn test_ui_state() {
    use rust_imperialism::turn_system::TurnPhase;
    use rust_imperialism::ui::state::{TurnState, UIState};

    let ui_state = UIState::default();
    assert!(ui_state.hero.is_none());
    assert_eq!(ui_state.monster_count, 0);

    // Test display text generation
    let ui_state = UIState {
        turn: TurnState {
            current_turn: 5,
            phase: TurnPhase::EnemyTurn,
        },
        monster_count: 3,
        ..Default::default()
    };

    assert_eq!(ui_state.turn_display_text(), "Turn: 5 - Enemy Turn");
    assert_eq!(ui_state.monster_count_text(), "Monsters: 3");
}

/// Test ECS World creation and entity spawning
#[test]
fn test_ecs_world_creation() {
    use rust_imperialism::health::{Combat, Health};
    use rust_imperialism::hero::Hero;
    use rust_imperialism::movement::ActionPoints;

    let mut world = World::new();

    // Spawn a hero entity
    let hero_entity = world
        .spawn((
            Hero::default(),
            Health::new(100),
            Combat::new(25),
            ActionPoints::new(6),
        ))
        .id();

    // Query the entity
    let hero = world.entity(hero_entity).get::<Hero>().unwrap();
    let health = world.entity(hero_entity).get::<Health>().unwrap();
    let combat = world.entity(hero_entity).get::<Combat>().unwrap();
    let ap = world.entity(hero_entity).get::<ActionPoints>().unwrap();

    assert!(!hero.is_selected);
    assert_eq!(health.current, 100);
    assert_eq!(combat.attack_damage, 25);
    assert_eq!(ap.current, 6);
}

/// Test monster behavior
#[test]
fn test_monster_behavior() {
    use rust_imperialism::health::Health;
    use rust_imperialism::monster::{Monster, MonsterBehavior};

    let mut monster = Monster {
        name: "Test Goblin".to_string(),
        sight_range: 5,
        behavior: MonsterBehavior::Aggressive,
    };

    let health = Health::new(30);
    assert_eq!(monster.behavior, MonsterBehavior::Aggressive);
    assert!(!monster.should_flee());

    // Test behavior change on low health
    let mut low_health = Health::new(30);
    low_health.take_damage(25); // Reduce to 5/30 = ~17% (below 25% threshold)
    monster.update_behavior_from_health(&low_health);
    assert_eq!(monster.behavior, MonsterBehavior::Fleeing);
    assert!(monster.should_flee());
}

/// Test combat damage calculations
#[test]
fn test_combat_calculations() {
    use rust_imperialism::health::{Combat, Health};

    let attacker = Combat::new(20);
    let mut defender = Health::new(50);

    // Apply damage
    defender.take_damage(attacker.attack_damage);
    assert_eq!(defender.current, 30);
    assert!(defender.is_alive());

    // Apply more damage
    defender.take_damage(attacker.attack_damage);
    assert_eq!(defender.current, 10);
    assert!(defender.is_alive());

    // Fatal damage
    defender.take_damage(attacker.attack_damage);
    assert_eq!(defender.current, 0);
    assert!(!defender.is_alive());
}

/// Test pathfinding basic types
#[test]
fn test_pathfinding_types() {
    use rust_imperialism::pathfinding::PathfindingSystem;

    // Test that the pathfinding system type exists and is accessible
    let system_name = std::any::type_name::<PathfindingSystem>();
    assert!(system_name.contains("PathfindingSystem"));
}

/// Test hex coordinate utilities
#[test]
fn test_hex_coordinates() {
    use bevy_ecs_tilemap::prelude::TilePos;
    use rust_imperialism::tile_pos::TilePosExt;

    let pos1 = TilePos { x: 1, y: 1 };
    let pos2 = TilePos { x: 2, y: 1 };

    let hex1 = pos1.to_hex();
    let hex2 = pos2.to_hex();

    let distance = hex1.distance_to(hex2);
    assert_eq!(distance, 1); // Adjacent tiles should have distance 1
}
