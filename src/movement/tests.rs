use super::*;
use crate::test_utils::*;
use bevy::prelude::*;
use std::collections::VecDeque;

#[test]
fn test_action_points_creation() {
    let ap = ActionPoints::new(6);
    assert_eq!(ap.current, 6);
    assert_eq!(ap.max, 6);
    assert!(!ap.is_exhausted());
}

#[test]
fn test_action_points_consumption() {
    let mut ap = ActionPoints::new(5);

    assert!(ap.can_move(3));
    ap.consume(3);
    assert_eq!(ap.current, 2);
    assert!(!ap.is_exhausted());

    assert!(ap.can_move(2));
    ap.consume(2);
    assert_eq!(ap.current, 0);
    assert!(ap.is_exhausted());

    // Cannot move with 0 points
    assert!(!ap.can_move(1));
}

#[test]
fn test_action_points_overconsume() {
    let mut ap = ActionPoints::new(3);

    // Consuming more than available should set to 0 (saturating_sub)
    ap.consume(5);
    assert_eq!(ap.current, 0);
    assert!(ap.is_exhausted());
}

#[test]
fn test_action_points_refresh() {
    let mut ap = ActionPoints::new(4);
    ap.consume(3);
    assert_eq!(ap.current, 1);

    ap.refresh();
    assert_eq!(ap.current, 4);
    assert_eq!(ap.max, 4);
    assert!(!ap.is_exhausted());
}

#[test]
fn test_movement_animation_creation() {
    let anim = MovementAnimation::new(200.0);
    assert_eq!(anim.movement_speed, 200.0);
    assert!(!anim.is_moving);
    assert!(anim.path.is_empty());
    assert!(anim.target_world_pos.is_none());
}

#[test]
fn test_movement_animation_start() {
    let mut anim = MovementAnimation::new(150.0);
    let target = Vec3::new(100.0, 50.0, 2.0);
    let path = VecDeque::from(vec![TilePos { x: 1, y: 1 }, TilePos { x: 2, y: 2 }]);

    anim.start_movement_to(target, path.clone());
    assert!(anim.is_moving);
    assert_eq!(anim.target_world_pos, Some(target));
    assert_eq!(anim.path, path);
}

#[test]
fn test_movement_types() {
    // Test that movement types can be created and compared
    let smart = MovementType::Smart;
    let simple = MovementType::Simple;

    // This mainly tests that the enum compiles and can be used
    match smart {
        MovementType::Smart => assert!(true),
        MovementType::Simple => assert!(false),
    }

    match simple {
        MovementType::Simple => assert!(true),
        MovementType::Smart => assert!(false),
    }
}

#[test]
fn test_move_entity_request() {
    let mut world = create_test_world();
    let entity = world.spawn_empty().id();
    let target = TilePos { x: 5, y: 5 };

    let request = MoveEntityRequest { entity, target };
    assert_eq!(request.entity, entity);
    assert_eq!(request.target, target);
}

#[test]
fn test_action_points_default() {
    let ap = ActionPoints::default();
    assert_eq!(ap.current, 6);
    assert_eq!(ap.max, 6);
}

#[test]
fn test_movement_animation_default() {
    let anim = MovementAnimation::default();
    assert_eq!(anim.movement_speed, 150.0);
    assert!(!anim.is_moving);
    assert!(anim.path.is_empty());
    assert!(anim.target_world_pos.is_none());
}

#[test]
fn test_action_points_edge_cases() {
    let mut world = create_test_world();
    let mut ap = ActionPoints::new(0);
    assert!(ap.is_exhausted());
    assert!(!ap.can_move(1));
    assert!(ap.can_move(0)); // Can move with 0 cost (0 >= 0)

    // Refresh should work even with 0 max
    ap.refresh();
    assert_eq!(ap.current, 0);
}

#[test]
fn test_movement_animation_empty_path() {
    let mut anim = MovementAnimation::new(100.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let empty_path = VecDeque::new();

    anim.start_movement_to(target, empty_path);
    assert!(anim.is_moving);
    assert!(anim.path.is_empty());
    assert_eq!(anim.target_world_pos, Some(target));
}
