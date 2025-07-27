use crate::health::{Combat, Health};
use crate::tile_pos::{HexExt, TilePosExt};
use crate::turn_system::TurnSystem;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;
use std::collections::VecDeque;

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

#[derive(Component, Debug, Clone)]
pub struct MonsterMovement {
    pub path: VecDeque<TilePos>,
    pub movement_speed: f32,
    pub is_moving: bool,
    pub target_world_pos: Option<Vec3>,
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

impl Default for MonsterMovement {
    fn default() -> Self {
        Self {
            path: VecDeque::new(),
            movement_speed: 150.0,
            is_moving: false,
            target_world_pos: None,
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
        app.add_systems(
            Update,
            (
                monster_ai_system,
                monster_movement_animation_system,
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

        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0..tilemap_size.x);
        let y = rng.gen_range(0..tilemap_size.y);
        let monster_pos = TilePos { x, y };
        let monster_world_pos = monster_pos.center_in_world(
            tilemap_size,
            grid_size,
            &TilemapTileSize { x: 16.0, y: 16.0 },
            map_type,
            &TilemapAnchor::Center,
        );

        let monster_types = ["Goblin", "Orc", "Skeleton"];
        let monster_name = monster_types[rng.gen_range(0..monster_types.len())];

        commands.spawn((
            Monster::new(monster_name.to_string(), turn_system.current_turn),
            Health::new(3),
            Combat::new(2),
            MonsterMovement::default(),
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
        println!(
            "Spawned {} at {:?} on turn {}",
            monster_name, monster_pos, turn_system.current_turn
        );
    }
}

fn monster_ai_system(
    mut monster_query: Query<
        (
            Entity,
            &mut Monster,
            &mut MonsterMovement,
            &TilePos,
            &Health,
        ),
        With<Monster>,
    >,
    hero_query: Query<(Entity, &TilePos), (With<crate::hero::Hero>, Without<Monster>)>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
    turn_system: Res<TurnSystem>,
    mut combat_events: EventWriter<crate::combat::CombatEvent>,
) {
    // Only allow monster AI during EnemyTurn phase
    if turn_system.phase != crate::turn_system::TurnPhase::EnemyTurn {
        return;
    }
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    let (hero_entity, hero_pos) = if let Ok((entity, pos)) = hero_query.get_single() {
        (entity, *pos)
    } else {
        return;
    };

    for (monster_entity, mut monster, mut movement, monster_pos, health) in monster_query.iter_mut()
    {
        // Update behavior based on health
        monster.update_behavior_from_health(health);
        if movement.is_moving {
            continue; // Skip if already moving
        }

        if monster.can_see_hero(*monster_pos, hero_pos) {
            let monster_hex = monster_pos.to_hex();
            let hero_hex = hero_pos.to_hex();
            let distance = monster_hex.distance_to(hero_hex) as u32;

            if monster.should_flee() {
                // Move away from hero
                move_monster_away_from_hero(
                    &mut movement,
                    *monster_pos,
                    hero_pos,
                    tilemap_size,
                    grid_size,
                    map_type,
                );
            } else if distance > 1 {
                // Move towards hero
                move_monster_towards_hero(
                    &mut movement,
                    *monster_pos,
                    hero_pos,
                    tilemap_size,
                    grid_size,
                    map_type,
                );
            } else if distance == 1 {
                // Monster is adjacent to hero - attack!
                combat_events.send(crate::combat::CombatEvent {
                    attacker: monster_entity,
                    defender: hero_entity,
                    damage: 2, // Default monster damage
                });
                println!("{} attacks the hero!", monster.name);
            }
        }
    }
}

fn move_monster_towards_hero(
    movement: &mut MonsterMovement,
    monster_pos: TilePos,
    hero_pos: TilePos,
    tilemap_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
) {
    let target_pos = get_next_position_towards(monster_pos, hero_pos, tilemap_size);
    if let Some(target) = target_pos {
        movement.path = vec![target].into();
        movement.target_world_pos = Some(
            target
                .center_in_world(
                    tilemap_size,
                    grid_size,
                    &TilemapTileSize { x: 16.0, y: 16.0 },
                    map_type,
                    &TilemapAnchor::Center,
                )
                .extend(1.0),
        );
        movement.is_moving = true;
    }
}

fn move_monster_away_from_hero(
    movement: &mut MonsterMovement,
    monster_pos: TilePos,
    hero_pos: TilePos,
    tilemap_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
) {
    let target_pos = get_next_position_away(monster_pos, hero_pos, tilemap_size);
    if let Some(target) = target_pos {
        movement.path = vec![target].into();
        movement.target_world_pos = Some(
            target
                .center_in_world(
                    tilemap_size,
                    grid_size,
                    &TilemapTileSize { x: 16.0, y: 16.0 },
                    map_type,
                    &TilemapAnchor::Center,
                )
                .extend(1.0),
        );
        movement.is_moving = true;
    }
}

fn get_next_position_towards(
    from: TilePos,
    to: TilePos,
    tilemap_size: &TilemapSize,
) -> Option<TilePos> {
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();

    // Get all neighbors of the from position
    let neighbors = from_hex.all_neighbors();

    // Find the neighbor closest to the target

    neighbors
        .into_iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| pos.x < tilemap_size.x && pos.y < tilemap_size.y)
        .min_by_key(|pos| {
            let pos_hex = pos.to_hex();
            pos_hex.distance_to(to_hex)
        })
}

fn get_next_position_away(
    from: TilePos,
    away_from: TilePos,
    tilemap_size: &TilemapSize,
) -> Option<TilePos> {
    let from_hex = from.to_hex();
    let away_hex = away_from.to_hex();

    // Get all neighbors and find the one farthest from the threat

    from_hex
        .all_neighbors()
        .into_iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| pos.x < tilemap_size.x && pos.y < tilemap_size.y)
        .max_by_key(|pos| {
            let pos_hex = pos.to_hex();
            pos_hex.distance_to(away_hex)
        })
}

fn monster_movement_animation_system(
    time: Res<Time>,
    mut monster_query: Query<(&mut Transform, &mut MonsterMovement, &mut TilePos), With<Monster>>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
) {
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for (mut transform, mut movement, mut monster_pos) in monster_query.iter_mut() {
        if !movement.is_moving {
            continue;
        }

        if let Some(target_pos) = movement.target_world_pos {
            let direction = target_pos - transform.translation;
            let distance = direction.length();

            if distance < 5.0 {
                transform.translation = target_pos;
                movement.is_moving = false;
                movement.target_world_pos = None;

                // Update logical position
                if let Some(next_tile) = movement.path.pop_front() {
                    *monster_pos = next_tile;
                }
            } else {
                let move_direction = direction.normalize();
                transform.translation +=
                    move_direction * movement.movement_speed * time.delta_secs();
            }
        }
    }
}
