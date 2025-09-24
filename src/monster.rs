use crate::health::{Combat, Health};
use crate::movement::{MoveEntityRequest, MovementAnimation, MovementPoints, MovementType};
use crate::tile_pos::{HexExt, TilePosExt};
use crate::turn_system::TurnSystem;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;

use crate::ui::logging::TerminalLogEvent;

#[derive(Component, Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub sight_range: u32,
    pub behavior: MonsterBehavior,
    pub spawn_turn: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MonsterBehavior {
    Aggressive, // Always attacks if hero in sight
    Defensive,  // Attacks if hero gets close
    Fleeing,    // Tries to retreat (low HP)
}

#[derive(Component)]
pub struct MonsterSprite;

impl Default for Monster {
    fn default() -> Self {
        Self {
            name: "Goblin".to_string(),
            sight_range: 5,
            behavior: MonsterBehavior::Aggressive,
            spawn_turn: 0,
        }
    }
}

impl Monster {
    pub fn new(name: String, spawn_turn: u32) -> Self {
        Self {
            name,
            sight_range: 5,
            behavior: MonsterBehavior::Aggressive,
            spawn_turn,
        }
    }

    pub fn should_flee(&self) -> bool {
        self.behavior == MonsterBehavior::Fleeing
    }

    pub fn can_see_hero(&self, monster_pos: TilePos, hero_pos: TilePos) -> bool {
        let monster_hex = monster_pos.to_hex();
        let hero_hex = hero_pos.to_hex();
        let distance = monster_hex.distance_to(hero_hex) as u32;
        distance <= self.sight_range
    }

    pub fn update_behavior_from_health(&mut self, health: &Health) {
        if health.is_low_health() {
            self.behavior = MonsterBehavior::Fleeing;
        }
    }
}

pub struct MonsterPlugin;

impl Plugin for MonsterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (monster_ai_system, spawn_monsters_system));
    }
}

fn spawn_monsters_system(
    mut commands: Commands,
    monster_query: Query<&Monster>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
    turn_system: Res<TurnSystem>,
    mut last_spawn_turn: Local<u32>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    // Only spawn if we have less than 5 monsters
    if monster_query.iter().count() >= 5 {
        return;
    }

    // Spawn every 3 turns
    if turn_system.current_turn > *last_spawn_turn && turn_system.current_turn.is_multiple_of(3) {
        let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
            return;
        };

        let mut rng = rand::rng();
        let x = rng.random_range(0..tilemap_size.x);
        let y = rng.random_range(0..tilemap_size.y);
        let monster_pos = TilePos { x, y };
        let monster_world_pos = monster_pos.center_in_world(
            tilemap_size,
            grid_size,
            &TilemapTileSize { x: 16.0, y: 16.0 },
            map_type,
            &TilemapAnchor::Center,
        );

        let monster_types = ["Goblin", "Orc", "Skeleton"];
        let monster_name = monster_types[rng.random_range(0..monster_types.len())];

        commands.spawn((
            Monster::new(monster_name.to_string(), turn_system.current_turn),
            Health::new(3),
            Combat::new(2),
            MovementPoints::new(2),        // Monsters have 2 movement points
            MovementAnimation::new(150.0), // Monster movement speed
            MovementType::Simple,          // Monsters use simple movement
            monster_pos,
            MonsterSprite,
            Sprite {
                color: Color::srgb(1.0, 0.0, 0.0), // Red color for monsters
                custom_size: Some(Vec2::new(10.0, 10.0)),
                ..default()
            },
            Transform::from_translation(monster_world_pos.extend(1.0)),
        ));

        *last_spawn_turn = turn_system.current_turn;
        log_writer.write(TerminalLogEvent {
            message: format!(
                "Spawned {} at {:?} on turn {}",
                monster_name, monster_pos, turn_system.current_turn
            ),
        });
    }
}

fn monster_ai_system(
    mut monster_query: Query<
        (
            Entity,
            &mut Monster,
            &MovementPoints,
            &MovementAnimation,
            &TilePos,
            &Health,
        ),
        With<Monster>,
    >,
    hero_query: Query<(Entity, &TilePos), (With<crate::hero::Hero>, Without<Monster>)>,
    turn_system: Res<TurnSystem>,
    mut combat_events: EventWriter<crate::combat::CombatEvent>,
    mut move_requests: EventWriter<MoveEntityRequest>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    // Only allow monster AI during EnemyTurn phase
    if turn_system.phase != crate::turn_system::TurnPhase::EnemyTurn {
        return;
    }

    let (hero_entity, hero_pos) = if let Ok((entity, pos)) = hero_query.single() {
        (entity, *pos)
    } else {
        return;
    };

    for (monster_entity, mut monster, movement_points, animation, monster_pos, health) in
        monster_query.iter_mut()
    {
        // Update behavior based on health
        monster.update_behavior_from_health(health);

        // Skip if already moving or out of movement points
        if animation.is_moving || movement_points.is_exhausted() {
            continue;
        }

        if monster.can_see_hero(*monster_pos, hero_pos) {
            let monster_hex = monster_pos.to_hex();
            let hero_hex = hero_pos.to_hex();
            let distance = monster_hex.distance_to(hero_hex) as u32;

            if monster.should_flee() {
                // Move away from hero
                let target = get_flee_target(*monster_pos, hero_pos);
                if let Some(target) = target {
                    move_requests.write(MoveEntityRequest {
                        entity: monster_entity,
                        target,
                    });
                }
            } else if distance > 1 {
                // Move towards hero
                move_requests.write(MoveEntityRequest {
                    entity: monster_entity,
                    target: hero_pos,
                });
            } else if distance == 1 {
                // Monster is adjacent to hero - attack!
                combat_events.write(crate::combat::CombatEvent {
                    attacker: monster_entity,
                    defender: hero_entity,
                    damage: 2, // Default monster damage
                });
                log_writer.write(TerminalLogEvent {
                    message: format!("{} attacks the hero!", monster.name),
                });
            }
        }
    }
}

/// Calculate a position to flee away from the hero
fn get_flee_target(monster_pos: TilePos, hero_pos: TilePos) -> Option<TilePos> {
    let monster_hex = monster_pos.to_hex();
    let hero_hex = hero_pos.to_hex();

    // Get all neighbors and find the one farthest from the hero
    monster_hex
        .all_neighbors()
        .into_iter()
        .filter_map(|hex| hex.to_tile_pos())
        .max_by_key(|pos| {
            let pos_hex = pos.to_hex();
            pos_hex.distance_to(hero_hex)
        })
}
