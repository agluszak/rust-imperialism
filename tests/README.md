# Testing Framework for Rust Imperialism

This directory contains the testing framework for the Rust Imperialism game, demonstrating effective ECS (Entity Component System) testing patterns using Bevy.

## Overview

The testing framework leverages ECS architecture to create isolated, fast, and reliable tests. Since ECS separates data (components) from logic (systems), we can easily test individual components and game mechanics without complex setup.

## Key Features

### ✅ **Isolated Component Testing**
```rust
#[test]
fn test_health_component() {
    let mut health = Health::new(100);
    health.take_damage(30);
    assert_eq!(health.current, 70);
    assert!(health.is_alive());
}
```

### ✅ **ECS World Testing**
```rust
#[test]
fn test_ecs_world_creation() {
    let mut world = World::new();
    let entity = world.spawn((
        Hero::default(),
        Health::new(100),
        ActionPoints::new(6),
    )).id();

    let health = world.entity(entity).get::<Health>().unwrap();
    assert_eq!(health.current, 100);
}
```

### ✅ **Game System Integration**
```rust
#[test]
fn test_turn_system() {
    let mut turn_system = TurnSystem::default();
    assert_eq!(turn_system.phase, TurnPhase::PlayerTurn);

    turn_system.advance_turn();
    assert_eq!(turn_system.phase, TurnPhase::Processing);
}
```

## Test Categories

### Unit Tests
- **Component Logic**: Health, Combat, ActionPoints
- **Game Mechanics**: Turn system, Movement, Pathfinding
- **Data Structures**: Tile properties, UI state

### Integration Tests
- **System Interactions**: Hero vs Monster combat
- **ECS Patterns**: Entity spawning and querying
- **Game Flow**: Multi-turn scenarios

## Benefits of ECS Testing

1. **Fast**: No graphics, audio, or file I/O
2. **Isolated**: Test individual components without dependencies
3. **Deterministic**: Pure logic testing without timing issues
4. **Comprehensive**: Easy to test edge cases and error conditions

## Running Tests

```bash
# Run all tests
cargo test

# Run specific integration test suite
cargo test --test core_mechanics
cargo test --test ai_civilian_commands
cargo test --test ai_resource_flow

# Run specific test
cargo test test_health_component

# Run with output
cargo test -- --nocapture
```

## Test Structure

```
tests/
├── common/                # Shared test utilities
│   └── mod.rs
├── core_mechanics.rs      # Basic system and utility tests
├── ai_civilian_commands.rs # AI command validation and execution tests
├── ai_resource_flow.rs    # Complex multi-turn AI behavior tests
└── README.md              # This documentation

src/
├── */tests.rs             # Unit tests for each module
├── test_utils.rs          # Testing utilities and helpers
└── lib.rs                 # Library exports for testing
```

## Best Practices

- **Test Behavior, Not Implementation**: Focus on what components do, not how they do it
- **Use Descriptive Test Names**: `test_hero_gains_experience_from_kills`
- **Test Edge Cases**: Zero values, maximum values, boundary conditions
- **Keep Tests Simple**: One concept per test
- **Use Helper Functions**: Create reusable test scenarios

## Example Test Patterns

### Component State Testing
```rust
#[test]
fn test_action_points_exhaustion() {
    let mut ap = ActionPoints::new(3);

    ap.consume(2);
    assert!(!ap.is_exhausted());

    ap.consume(1);
    assert!(ap.is_exhausted());
    assert!(!ap.can_move(1));
}
```

### System Behavior Testing
```rust
#[test]
fn test_monster_behavior_change() {
    let mut monster = Monster::new("Goblin".to_string());
    let mut health = Health::new(20);

    health.take_damage(16); // Reduce to low health
    monster.update_behavior_from_health(&health);

    assert!(monster.should_flee());
}
```

This framework makes it easy to ensure game mechanics work correctly and catch regressions early in development.