use bevy::prelude::*;

use crate::health::Health;
use crate::hero::Hero;
use crate::turn_system::TurnSystem;
use crate::ui::components::{HeroStatusDisplay, TurnDisplay};

pub fn update_turn_display(
    turn_system: Res<TurnSystem>,
    mut query: Query<&mut Text, With<TurnDisplay>>,
) {
    if turn_system.is_changed() {
        for mut text in query.iter_mut() {
            let phase_text = match turn_system.phase {
                crate::turn_system::TurnPhase::PlayerTurn => "Player Turn",
                crate::turn_system::TurnPhase::Processing => "Processing",
                crate::turn_system::TurnPhase::EnemyTurn => "Enemy Turn",
            };
            text.0 = format!("Turn: {} - {}", turn_system.current_turn, phase_text);
        }
    }
}

pub fn update_hero_status_display(
    hero_query: Query<
        (&Hero, &Health, &crate::movement::ActionPoints),
        (
            With<Hero>,
            Or<(
                Changed<Hero>,
                Changed<Health>,
                Changed<crate::movement::ActionPoints>,
            )>,
        ),
    >,
    mut text_query: Query<&mut Text, With<HeroStatusDisplay>>,
) {
    for (hero, health, action_points) in hero_query.iter() {
        for mut text in text_query.iter_mut() {
            let selection_text = if hero.is_selected { " [SELECTED]" } else { "" };
            text.0 = format!(
                "Hero: HP {}/{}, AP {}/{}, Kills: {}{}",
                health.current,
                health.max,
                action_points.current,
                action_points.max,
                hero.kills,
                selection_text
            );
        }
    }
}
