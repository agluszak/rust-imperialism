use crate::health::{Combat, Health};
use crate::hero::Hero;
use crate::monster::Monster;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

#[derive(Event)]
pub struct CombatEvent {
    pub attacker: Entity,
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

// Removed auto_combat_system - combat is now manual and costs MP

fn process_combat_events(
    mut combat_events: EventReader<CombatEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut health_query: Query<&mut Health>,
    hero_query: Query<&Hero>,
    monster_query: Query<&Monster>,
) {
    for event in combat_events.read() {
        // Apply damage to defender
        if let Ok(mut health) = health_query.get_mut(event.defender) {
            health.take_damage(event.damage);

            // Check if this is a hero or monster for different logging
            if let Ok(hero) = hero_query.get(event.defender) {
                println!(
                    "Hero takes {} damage! HP: {}/{}",
                    event.damage, health.current, health.max
                );

                if !health.is_alive() {
                    death_events.send(DeathEvent {
                        entity: event.defender,
                        was_monster: false,
                    });
                }
            } else if let Ok(monster) = monster_query.get(event.defender) {
                println!(
                    "{} takes {} damage! HP: {}/{}",
                    monster.name, event.damage, health.current, health.max
                );

                if !health.is_alive() {
                    death_events.send(DeathEvent {
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
) {
    for event in death_events.read() {
        if event.was_monster {
            // Monster died - remove it and give hero a kill
            if let Ok(monster) = monster_query.get(event.entity) {
                println!("{} has been defeated!", monster.name);
            }

            commands.entity(event.entity).despawn();

            // Give hero a kill
            if let Ok((mut hero, mut health)) = hero_query.get_single_mut() {
                hero.add_kill();
                // Heal after every 3 kills
                if hero.should_heal_from_kills() {
                    health.heal_to_full();
                    println!("Hero healed to full HP after {} kills!", hero.kills);
                }
            }
        } else {
            // Hero died - game over
            println!("GAME OVER - Hero has been defeated!");
            // You could add game over logic here
        }
    }
}

// Helper function to initiate combat when hero attacks
pub fn initiate_hero_attack(
    hero_entity: Entity,
    target_pos: TilePos,
    hero_query: &mut Query<&mut Hero>,
    hero_combat_query: &Query<&Combat, With<Hero>>,
    monster_query: &Query<(Entity, &Monster, &TilePos), With<Monster>>,
    combat_events: &mut EventWriter<CombatEvent>,
) -> bool {
    let Ok(mut hero) = hero_query.get_mut(hero_entity) else {
        return false;
    };

    if !hero.can_attack() {
        println!("Hero doesn't have enough movement points to attack!");
        return false;
    }

    // Find monster at target position
    for (monster_entity, monster, monster_pos) in monster_query.iter() {
        if *monster_pos == target_pos && hero.attack() {
            let damage = if let Ok(combat) = hero_combat_query.get(hero_entity) {
                combat.attack_damage
            } else {
                3 // Fallback damage
            };

            combat_events.send(CombatEvent {
                attacker: hero_entity,
                defender: monster_entity,
                damage,
            });
            println!("Hero attacks {}!", monster.name);
            return true;
        }
    }

    false
}

// Hero attack system
fn hero_attack_system(
    mut hero_attack_events: EventReader<HeroAttackClicked>,
    mut hero_query: Query<(Entity, &mut Hero, &mut crate::hero::HeroPathPreview), With<Hero>>,
    hero_combat_query: Query<&Combat, With<Hero>>,
    monster_query: Query<(Entity, &Monster, &TilePos), With<Monster>>,
    mut combat_events: EventWriter<CombatEvent>,
) {
    for event in hero_attack_events.read() {
        // Find the monster at target position
        let monster_info = monster_query
            .iter()
            .find(|(_, _, monster_pos)| **monster_pos == event.target_pos)
            .map(|(entity, monster, _)| (entity, monster.name.clone()));

        if let Some((monster_entity, monster_name)) = monster_info {
            // Find selected hero
            for (hero_entity, mut hero, mut path_preview) in hero_query.iter_mut() {
                if hero.is_selected {
                    if hero.can_attack() {
                        hero.attack();
                        path_preview.clear();

                        let damage = if let Ok(combat) = hero_combat_query.get(hero_entity) {
                            combat.attack_damage
                        } else {
                            3 // Fallback damage
                        };

                        combat_events.send(CombatEvent {
                            attacker: hero_entity,
                            defender: monster_entity,
                            damage,
                        });
                        println!("Hero attacks {monster_name}!");
                    } else {
                        println!("Hero doesn't have enough movement points to attack!");
                    }
                    break;
                }
            }
        }
    }
}
