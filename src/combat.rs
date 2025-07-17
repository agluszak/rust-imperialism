use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use crate::hero::Hero;
use crate::monster::Monster;

#[derive(Event)]
pub struct CombatEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub damage: u32,
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
            .add_event::<DeathEvent>()
            .add_systems(Update, (
                auto_combat_system,
                process_combat_events,
                process_death_events,
            ));
    }
}

fn auto_combat_system(
    mut combat_events: EventWriter<CombatEvent>,
    hero_query: Query<(Entity, &TilePos), (With<Hero>, Without<Monster>)>,
    monster_query: Query<(Entity, &Monster, &TilePos), (With<Monster>, Without<Hero>)>,
) {
    let Ok((hero_entity, hero_pos)) = hero_query.get_single() else {
        return;
    };

    // Check for adjacent monsters to fight
    for (monster_entity, monster, monster_pos) in monster_query.iter() {
        let distance = ((hero_pos.x as i32 - monster_pos.x as i32).abs() + 
                       (hero_pos.y as i32 - monster_pos.y as i32).abs()) as u32;

        if distance == 1 {
            // Monster attacks hero
            combat_events.send(CombatEvent {
                attacker: monster_entity,
                defender: hero_entity,
                damage: monster.attack_damage,
            });

            // Hero counter-attacks
            combat_events.send(CombatEvent {
                attacker: hero_entity,
                defender: monster_entity,
                damage: 3, // Hero damage
            });
        }
    }
}

fn process_combat_events(
    mut combat_events: EventReader<CombatEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut hero_query: Query<&mut Hero>,
    mut monster_query: Query<&mut Monster>,
) {
    for event in combat_events.read() {
        // Apply damage to defender
        if let Ok(mut hero) = hero_query.get_mut(event.defender) {
            hero.take_damage(event.damage);
            println!("Hero takes {} damage! HP: {}/{}", event.damage, hero.hp, hero.max_hp);
            
            if !hero.is_alive() {
                death_events.send(DeathEvent {
                    entity: event.defender,
                    was_monster: false,
                });
            }
        } else if let Ok(mut monster) = monster_query.get_mut(event.defender) {
            monster.take_damage(event.damage);
            println!("{} takes {} damage! HP: {}/{}", monster.name, event.damage, monster.hp, monster.max_hp);
            
            if !monster.is_alive() {
                death_events.send(DeathEvent {
                    entity: event.defender,
                    was_monster: true,
                });
            }
        }
    }
}

fn process_death_events(
    mut death_events: EventReader<DeathEvent>,
    mut commands: Commands,
    mut hero_query: Query<&mut Hero>,
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
            if let Ok(mut hero) = hero_query.get_single_mut() {
                hero.add_kill();
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
        if *monster_pos == target_pos {
            if hero.attack() {
                combat_events.send(CombatEvent {
                    attacker: hero_entity,
                    defender: monster_entity,
                    damage: 3,
                });
                println!("Hero attacks {}!", monster.name);
                return true;
            }
        }
    }

    false
}