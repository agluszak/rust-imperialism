use crate::health::{Combat, Health};
use crate::hero::Hero;
use crate::monster::Monster;
use crate::tile_pos::TilePosExt;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::ui::logging::TerminalLogEvent;

#[derive(Event)]
pub struct CombatEvent {
    pub _attacker: Entity,
    pub defender: Entity,
    pub damage: u32,
}

// Events for combat input
#[derive(Event)]
pub struct HeroAttackClicked {
    pub target_pos: TilePos,
}

#[derive(Event)]
pub struct DeathEvent {
    pub entity: Entity,
    pub was_monster: bool,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CombatEvent>()
            .add_event::<HeroAttackClicked>()
            .add_event::<DeathEvent>()
            .add_systems(
                Update,
                (
                    process_combat_events,
                    process_death_events,
                    hero_attack_system,
                ),
            );
    }
}

// Removed auto_combat_system - combat is now manual and costs AP

fn process_combat_events(
    mut combat_events: EventReader<CombatEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut health_query: Query<&mut Health>,
    hero_query: Query<&Hero>,
    monster_query: Query<&Monster>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    for event in combat_events.read() {
        // Apply damage to defender
        if let Ok(mut health) = health_query.get_mut(event.defender) {
            health.take_damage(event.damage);

            // Check if this is a hero or monster for different logging
            if let Ok(_hero) = hero_query.get(event.defender) {
                log_writer.write(TerminalLogEvent {
                    message: format!(
                        "Hero takes {} damage! HP: {}/{}",
                        event.damage, health.current, health.max
                    ),
                });

                if !health.is_alive() {
                    death_events.write(DeathEvent {
                        entity: event.defender,
                        was_monster: false,
                    });
                }
            } else if let Ok(monster) = monster_query.get(event.defender) {
                log_writer.write(TerminalLogEvent {
                    message: format!(
                        "{} takes {} damage! HP: {}/{}",
                        monster.name, event.damage, health.current, health.max
                    ),
                });

                if !health.is_alive() {
                    death_events.write(DeathEvent {
                        entity: event.defender,
                        was_monster: true,
                    });
                }
            }
        }
    }
}

fn process_death_events(
    mut death_events: EventReader<DeathEvent>,
    mut commands: Commands,
    mut hero_query: Query<(&mut Hero, &mut Health), With<Hero>>,
    monster_query: Query<&Monster>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    for event in death_events.read() {
        if event.was_monster {
            // Monster died - remove it and give hero a kill
            if let Ok(monster) = monster_query.get(event.entity) {
                log_writer.write(TerminalLogEvent {
                    message: format!("{} has been defeated!", monster.name),
                });
            }

            commands.entity(event.entity).despawn();

            // Give hero a kill
            if let Ok((mut hero, mut health)) = hero_query.single_mut() {
                hero.add_kill();
                // Heal after every 3 kills
                if hero.should_heal_from_kills() {
                    health.heal_to_full();
                    log_writer.write(TerminalLogEvent {
                        message: format!("Hero healed to full HP after {} kills!", hero.kills),
                    });
                }
            }
        } else {
            // Hero died - game over
            log_writer.write(TerminalLogEvent {
                message: "GAME OVER - Hero has been defeated!".to_string(),
            });
            // You could add game over logic here
        }
    }
}

// Hero attack system
fn hero_attack_system(
    mut hero_attack_events: EventReader<HeroAttackClicked>,
    mut hero_query: Query<
        (
            Entity,
            &mut Hero,
            &mut crate::movement::ActionPoints,
            &mut crate::hero::HeroPathPreview,
            &TilePos,
        ),
        With<Hero>,
    >,
    hero_combat_query: Query<&Combat, With<Hero>>,
    monster_query: Query<(Entity, &Monster, &TilePos), With<Monster>>,
    mut combat_events: EventWriter<CombatEvent>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    for event in hero_attack_events.read() {
        // Find the monster at target position
        let monster_info = monster_query
            .iter()
            .find(|(_, _, monster_pos)| **monster_pos == event.target_pos)
            .map(|(entity, monster, _)| (entity, monster.name.clone()));

        if let Some((monster_entity, monster_name)) = monster_info {
            // Find selected hero
            for (hero_entity, hero, mut action_points, mut path_preview, hero_pos) in
                hero_query.iter_mut()
            {
                if hero.is_selected {
                    // Check if hero is adjacent to the monster (attack range = 1)
                    let hero_hex = hero_pos.to_hex();
                    let monster_hex = event.target_pos.to_hex();
                    let distance = hero_hex.distance_to(monster_hex);

                    if distance > 1 {
                        log_writer.write(TerminalLogEvent { message: format!(
                            "Monster is too far away! Hero must be adjacent to attack (distance: {})",
                            distance
                        )});
                        break;
                    }

                    if action_points.can_move(1) {
                        action_points.consume(1); // Attack costs 1 AP
                        path_preview.clear();

                        let damage = if let Ok(combat) = hero_combat_query.get(hero_entity) {
                            combat.attack_damage
                        } else {
                            3 // Fallback damage
                        };

                        combat_events.write(CombatEvent {
                            _attacker: hero_entity,
                            defender: monster_entity,
                            damage,
                        });
                        log_writer.write(TerminalLogEvent {
                            message: format!("Hero attacks {monster_name}!"),
                        });
                    } else {
                        log_writer.write(TerminalLogEvent {
                            message: "Hero doesn't have enough action points to attack!"
                                .to_string(),
                        });
                    }
                    break;
                }
            }
        }
    }
}
