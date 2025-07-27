use crate::health::Health;
use crate::hero::Hero;
use crate::turn_system::TurnSystem;
use bevy::prelude::*;

#[derive(Component)]
pub struct TurnDisplay;

#[derive(Component)]
pub struct HeroStatusDisplay;

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, (update_turn_display, update_hero_status_display));
    }
}

fn setup_ui(mut commands: Commands) {
    // Create UI root
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // Turn display
            parent.spawn((
                Text::new("Turn: 1 - Player Turn"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TurnDisplay,
            ));

            // Hero status display
            parent.spawn((
                Text::new("Hero: HP 10/10, MP 3/3, Kills: 0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                HeroStatusDisplay,
            ));
        });
}

fn update_turn_display(
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

fn update_hero_status_display(
    hero_query: Query<(&Hero, &Health), (With<Hero>, Or<(Changed<Hero>, Changed<Health>)>)>,
    mut text_query: Query<&mut Text, With<HeroStatusDisplay>>,
) {
    for (hero, health) in hero_query.iter() {
        for mut text in text_query.iter_mut() {
            let selection_text = if hero.is_selected { " [SELECTED]" } else { "" };
            text.0 = format!(
                "Hero: HP {}/{}, MP {}/{}, Kills: {}{}",
                health.current,
                health.max,
                hero.movement_points,
                hero.max_movement_points,
                hero.kills,
                selection_text
            );
        }
    }
}
