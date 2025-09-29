#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::Health;
    use crate::hero::Hero;
    use crate::movement::ActionPoints;
    use crate::test_utils::*;
    use crate::test_with_world;
    use crate::turn_system::{TurnPhase, TurnSystem};
    use bevy_ecs_tilemap::prelude::*;

    #[test]
    fn test_ui_state_default() {
        let ui_state = UIState::default();
        assert!(ui_state.hero.is_none());
        assert_eq!(ui_state.turn.current_turn, 1);
        assert_eq!(ui_state.turn.phase, TurnPhase::PlayerTurn);
        assert_eq!(ui_state.monster_count, 0);
    }

    #[test]
    fn test_health_state_conversion() {
        let health = Health::new(80);
        let health_state = HealthState::from(&health);

        assert_eq!(health_state.current, 80);
        assert_eq!(health_state.max, 80);
    }

    #[test]
    fn test_action_points_state_conversion() {
        let ap = ActionPoints::new(6);
        let ap_state = ActionPointsState::from(&ap);

        assert_eq!(ap_state.current, 6);
        assert_eq!(ap_state.max, 6);
    }

    #[test]
    fn test_turn_state_conversion() {
        let mut turn_system = TurnSystem::default();
        turn_system.advance_turn(); // Move to Processing

        let turn_state = TurnState::from(&turn_system);
        assert_eq!(turn_state.current_turn, 1);
        assert_eq!(turn_state.phase, TurnPhase::Processing);
    }

    test_with_world!(test_ui_state_update_with_hero, {
        let mut ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        // Create hero data
        let hero = Hero {
            name: "Test Hero".to_string(),
            is_selected: true,
            kills: 3,
        };
        let health = Health::new(100);
        let mut action_points = ActionPoints::new(6);
        action_points.consume(2);
        let position = TilePos { x: 5, y: 10 };

        let hero_data = Some((&hero, &health, &action_points, &position));
        ui_state.update(hero_data, &turn_system, 2);

        assert!(ui_state.hero.is_some());
        let hero_state = ui_state.hero.as_ref().unwrap();
        assert!(hero_state.is_selected);
        assert_eq!(hero_state.health.current, 100);
        assert_eq!(hero_state.health.max, 100);
        assert_eq!(hero_state.action_points.current, 4);
        assert_eq!(hero_state.action_points.max, 6);
        assert_eq!(hero_state.kills, 3);
        assert_eq!(hero_state.position, position);
        assert_eq!(ui_state.monster_count, 2);
    });

    test_with_world!(test_ui_state_update_without_hero, {
        let mut ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        ui_state.update(None, &turn_system, 1);

        assert!(ui_state.hero.is_none());
        assert_eq!(ui_state.monster_count, 1);
        assert_eq!(ui_state.turn.current_turn, 1);
        assert_eq!(ui_state.turn.phase, TurnPhase::PlayerTurn);
    });

    #[test]
    fn test_ui_state_needs_update_turn_change() {
        let ui_state = UIState::default();
        let mut turn_system = TurnSystem::default();
        turn_system.advance_turn(); // Change turn

        assert!(ui_state.needs_update(None, &turn_system, 0));
    }

    #[test]
    fn test_ui_state_needs_update_monster_count_change() {
        let ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        assert!(ui_state.needs_update(None, &turn_system, 3)); // Monster count changed from 0 to 3
    }

    test_with_world!(test_ui_state_needs_update_hero_changes, {
        let mut ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        // Set up initial state with hero
        let hero = Hero::default();
        let health = Health::new(100);
        let action_points = ActionPoints::new(6);
        let position = TilePos { x: 0, y: 0 };

        let hero_data = (&hero, &health, &action_points, &position);
        ui_state.update(Some(hero_data), &turn_system, 0);

        // Test health change
        let mut changed_health = health.clone();
        changed_health.take_damage(50);
        let changed_hero_data = (&hero, &changed_health, &action_points, &position);
        assert!(ui_state.needs_update(Some(changed_hero_data), &turn_system, 0));

        // Test action points change
        let mut changed_ap = action_points.clone();
        changed_ap.consume(3);
        let changed_hero_data = (&hero, &health, &changed_ap, &position);
        assert!(ui_state.needs_update(Some(changed_hero_data), &turn_system, 0));

        // Test position change
        let new_position = TilePos { x: 1, y: 1 };
        let changed_hero_data = (&hero, &health, &action_points, &new_position);
        assert!(ui_state.needs_update(Some(changed_hero_data), &turn_system, 0));
    });

    #[test]
    fn test_ui_state_no_update_needed() {
        let ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        // Same state should not need update
        assert!(!ui_state.needs_update(None, &turn_system, 0));
    }

    #[test]
    fn test_turn_display_text() {
        let ui_state = UIState {
            turn: TurnState {
                current_turn: 5,
                phase: TurnPhase::EnemyTurn,
            },
            ..Default::default()
        };

        assert_eq!(ui_state.turn_display_text(), "Turn: 5 - Enemy Turn");
    }

    #[test]
    fn test_hero_status_text_with_hero() {
        let ui_state = UIState {
            hero: Some(HeroState {
                is_selected: true,
                health: HealthState {
                    current: 75,
                    max: 100,
                },
                action_points: ActionPointsState { current: 3, max: 6 },
                kills: 7,
                position: TilePos { x: 0, y: 0 },
            }),
            ..Default::default()
        };

        assert_eq!(
            ui_state.hero_status_text(),
            "Hero: HP 75/100, AP 3/6, Kills: 7 [SELECTED]"
        );
    }

    #[test]
    fn test_hero_status_text_without_hero() {
        let ui_state = UIState::default();
        assert_eq!(ui_state.hero_status_text(), "No Hero");
    }

    #[test]
    fn test_hero_status_text_not_selected() {
        let ui_state = UIState {
            hero: Some(HeroState {
                is_selected: false,
                health: HealthState {
                    current: 100,
                    max: 100,
                },
                action_points: ActionPointsState { current: 6, max: 6 },
                kills: 0,
                position: TilePos { x: 0, y: 0 },
            }),
            ..Default::default()
        };

        assert_eq!(
            ui_state.hero_status_text(),
            "Hero: HP 100/100, AP 6/6, Kills: 0"
        );
    }

    #[test]
    fn test_monster_count_text() {
        let ui_state = UIState {
            monster_count: 42,
            ..Default::default()
        };

        assert_eq!(ui_state.monster_count_text(), "Monsters: 42");
    }

    #[test]
    fn test_turn_state_default() {
        let turn_state = TurnState::default();
        assert_eq!(turn_state.current_turn, 1);
        assert_eq!(turn_state.phase, TurnPhase::PlayerTurn);
    }

    test_with_world!(test_ui_state_hero_appearance_disappearance, {
        let mut ui_state = UIState::default();
        let turn_system = TurnSystem::default();

        // Initially no hero
        assert!(!ui_state.needs_update(None, &turn_system, 0));

        // Hero appears
        let hero = Hero::default();
        let health = Health::new(100);
        let action_points = ActionPoints::new(6);
        let position = TilePos { x: 0, y: 0 };
        let hero_data = (&hero, &health, &action_points, &position);

        assert!(ui_state.needs_update(Some(hero_data), &turn_system, 0));
        ui_state.update(Some(hero_data), &turn_system, 0);

        // Hero disappears
        assert!(ui_state.needs_update(None, &turn_system, 0));
    });

    #[test]
    fn test_ui_state_multiple_turn_phases() {
        let mut ui_state = UIState::default();

        let phases = [
            (TurnPhase::PlayerTurn, "Player Turn"),
            (TurnPhase::Processing, "Processing"),
            (TurnPhase::EnemyTurn, "Enemy Turn"),
        ];

        for (phase, expected_text) in phases {
            ui_state.turn.phase = phase;
            ui_state.turn.current_turn = 10;

            let display_text = ui_state.turn_display_text();
            assert!(display_text.contains("Turn: 10"));
            assert!(display_text.contains(expected_text));
        }
    }
}
