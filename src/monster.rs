use crate::constants::*;
use crate::health::{Combat, Health};
use crate::movement::{ActionPoints, MoveEntityRequest, MovementAnimation, MovementType};
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
            sight_range: MONSTER_SIGHT_RANGE,
            behavior: MonsterBehavior::Aggressive,
        }
    }
}

impl Monster {
    pub fn new(name: String) -> Self {
        Self {
            name,
            sight_range: MONSTER_SIGHT_RANGE,
            behavior: MonsterBehavior::Aggressive,
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
        app.add_systems(
            Update,
            (
                refresh_monster_action_points_system,
                monster_ai_system,
                spawn_monsters_system,
            ),
        );
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
    // Only spawn if we have less than max monsters
    if monster_query.iter().count() >= MAX_MONSTERS {
        return;
    }

    // Spawn every N turns
    if turn_system.current_turn > *last_spawn_turn
        && turn_system
            .current_turn
            .is_multiple_of(MONSTER_SPAWN_INTERVAL)
    {
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
            &TilemapTileSize {
                x: TILE_SIZE,
                y: TILE_SIZE,
            },
            map_type,
            &TilemapAnchor::Center,
        );

        let monster_types = ["Goblin", "Orc", "Skeleton"];
        let monster_name = monster_types[rng.random_range(0..monster_types.len())];

        commands.spawn((
            Monster::new(monster_name.to_string()),
            Health::new(MONSTER_MAX_HEALTH),
            Combat::new(MONSTER_DAMAGE),
            ActionPoints::new(MONSTER_MAX_ACTION_POINTS),
            MovementAnimation::new(MONSTER_MOVEMENT_SPEED),
            MovementType::Simple,
            monster_pos,
            MonsterSprite,
            Sprite {
                color: MONSTER_COLOR,
                custom_size: Some(MONSTER_SPRITE_SIZE),
                ..default()
            },
            Transform::from_translation(monster_world_pos.extend(Z_LAYER_MONSTERS)),
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

fn refresh_monster_action_points_system(
    mut monster_query: Query<&mut ActionPoints, With<Monster>>,
    turn_system: Res<TurnSystem>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    // Refresh monster action points when the phase changes to EnemyTurn
    if turn_system.is_changed() && turn_system.phase == crate::turn_system::TurnPhase::EnemyTurn {
        for mut action_points in monster_query.iter_mut() {
            let old_ap = action_points.current;
            action_points.refresh();

            if old_ap < action_points.current {
                log_writer.write(TerminalLogEvent {
                    message: format!(
                        "Monster action points refreshed: {}/{}",
                        action_points.current, action_points.max
                    ),
                });
            }
        }
    }
}

fn monster_ai_system(
    mut monster_query: Query<
        (
            Entity,
            &mut Monster,
            &mut ActionPoints,
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

    for (monster_entity, mut monster, mut action_points, animation, monster_pos, health) in
        monster_query.iter_mut()
    {
        // Update behavior based on health
        monster.update_behavior_from_health(health);

        // Skip if already moving or out of action points
        if animation.is_moving || action_points.is_exhausted() {
            continue;
        }

        if monster.can_see_hero(*monster_pos, hero_pos) {
            process_monster_action(
                monster_entity,
                &monster,
                &mut action_points,
                *monster_pos,
                hero_entity,
                hero_pos,
                &mut move_requests,
                &mut combat_events,
                &mut log_writer,
            );
        }
    }
}

/// Process a single monster's action during their turn
fn process_monster_action(
    monster_entity: Entity,
    monster: &Monster,
    action_points: &mut ActionPoints,
    monster_pos: TilePos,
    hero_entity: Entity,
    hero_pos: TilePos,
    move_requests: &mut EventWriter<MoveEntityRequest>,
    combat_events: &mut EventWriter<crate::combat::CombatEvent>,
    log_writer: &mut EventWriter<TerminalLogEvent>,
) {
    let distance = calculate_distance(monster_pos, hero_pos);

    if monster.should_flee() {
        handle_flee_action(monster_entity, monster_pos, hero_pos, move_requests);
    } else if distance > MONSTER_ATTACK_RANGE {
        handle_move_action(monster_entity, hero_pos, move_requests);
    } else {
        handle_attack_action(
            monster_entity,
            hero_entity,
            &monster.name,
            action_points,
            combat_events,
            log_writer,
        );
    }
}

/// Calculate the distance between two tile positions
fn calculate_distance(from: TilePos, to: TilePos) -> u32 {
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();
    from_hex.distance_to(to_hex) as u32
}

/// Handle fleeing from the hero
fn handle_flee_action(
    monster_entity: Entity,
    monster_pos: TilePos,
    hero_pos: TilePos,
    move_requests: &mut EventWriter<MoveEntityRequest>,
) {
    if let Some(target) = calculate_optimal_flee_position(monster_pos, hero_pos) {
        move_requests.write(MoveEntityRequest {
            entity: monster_entity,
            target,
        });
    }
}

/// Handle moving towards the hero
fn handle_move_action(
    monster_entity: Entity,
    hero_pos: TilePos,
    move_requests: &mut EventWriter<MoveEntityRequest>,
) {
    move_requests.write(MoveEntityRequest {
        entity: monster_entity,
        target: hero_pos,
    });
}

/// Handle attacking the hero
fn handle_attack_action(
    monster_entity: Entity,
    hero_entity: Entity,
    monster_name: &str,
    action_points: &mut ActionPoints,
    combat_events: &mut EventWriter<crate::combat::CombatEvent>,
    log_writer: &mut EventWriter<TerminalLogEvent>,
) {
    if action_points.can_move(ATTACK_ACTION_COST) {
        action_points.consume(ATTACK_ACTION_COST);
        combat_events.write(crate::combat::CombatEvent {
            attacker: monster_entity,
            defender: hero_entity,
            damage: MONSTER_DAMAGE,
        });
        log_writer.write(TerminalLogEvent {
            message: format!("{} attacks the hero!", monster_name),
        });
    } else {
        log_writer.write(TerminalLogEvent {
            message: format!("{} wants to attack but has no action points!", monster_name),
        });
    }
}

/// Calculate the optimal position to flee away from the hero
fn calculate_optimal_flee_position(monster_pos: TilePos, hero_pos: TilePos) -> Option<TilePos> {
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
