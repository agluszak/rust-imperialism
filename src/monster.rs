use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;
use std::collections::VecDeque;

#[derive(Component, Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub hp: u32,
    pub max_hp: u32,
    pub attack_damage: u32,
    pub sight_range: u32,
    pub behavior: MonsterBehavior,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MonsterBehavior {
    Aggressive,  // Always attacks if hero in sight
    Defensive,   // Attacks if hero gets close
    Fleeing,     // Tries to retreat (low HP)
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
            hp: 3,
            max_hp: 3,
            attack_damage: 2,
            sight_range: 5,
            behavior: MonsterBehavior::Aggressive,
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
    pub fn new(name: String, hp: u32, attack_damage: u32) -> Self {
        Self {
            name,
            hp,
            max_hp: hp,
            attack_damage,
            sight_range: 5,
            behavior: MonsterBehavior::Aggressive,
        }
    }

    pub fn take_damage(&mut self, damage: u32) {
        self.hp = self.hp.saturating_sub(damage);
        
        // Switch to fleeing behavior if HP is low
        if self.hp <= self.max_hp / 3 {
            self.behavior = MonsterBehavior::Fleeing;
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn should_flee(&self) -> bool {
        self.behavior == MonsterBehavior::Fleeing
    }

    pub fn can_see_hero(&self, monster_pos: TilePos, hero_pos: TilePos) -> bool {
        let distance = ((monster_pos.x as i32 - hero_pos.x as i32).abs() + 
                       (monster_pos.y as i32 - hero_pos.y as i32).abs()) as u32;
        distance <= self.sight_range
    }
}

pub struct MonsterPlugin;

impl Plugin for MonsterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            monster_ai_system,
            monster_movement_system,
            spawn_monsters_system,
        ));
    }
}

fn spawn_monsters_system(
    mut commands: Commands,
    monster_query: Query<&Monster>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
    mut spawn_timer: Local<Timer>,
    time: Res<Time>,
) {
    // Only spawn if we have less than 5 monsters
    if monster_query.iter().count() >= 5 {
        return;
    }

    if spawn_timer.duration().is_zero() {
        *spawn_timer = Timer::from_seconds(5.0, TimerMode::Repeating);
    }

    spawn_timer.tick(time.delta());

    if spawn_timer.just_finished() {
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
            Monster::new(monster_name.to_string(), 3, 2),
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

        println!("Spawned {} at {:?}", monster_name, monster_pos);
    }
}

fn monster_ai_system(
    mut monster_query: Query<(&mut Monster, &mut MonsterMovement, &TilePos), With<Monster>>,
    hero_query: Query<&TilePos, (With<crate::hero::Hero>, Without<Monster>)>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
) {
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };

    let hero_pos = if let Ok(pos) = hero_query.get_single() {
        *pos
    } else {
        return;
    };

    for (mut monster, mut movement, monster_pos) in monster_query.iter_mut() {
        if movement.is_moving {
            continue; // Skip if already moving
        }

        if monster.can_see_hero(*monster_pos, hero_pos) {
            let distance = ((monster_pos.x as i32 - hero_pos.x as i32).abs() + 
                           (monster_pos.y as i32 - hero_pos.y as i32).abs()) as u32;

            if monster.should_flee() {
                // Move away from hero
                move_monster_away_from_hero(&mut movement, *monster_pos, hero_pos, tilemap_size, grid_size, map_type);
            } else if distance > 1 {
                // Move towards hero
                move_monster_towards_hero(&mut movement, *monster_pos, hero_pos, tilemap_size, grid_size, map_type);
            }
            // If distance == 1, monster is adjacent and will attack in combat system
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
            target.center_in_world(tilemap_size, grid_size, &TilemapTileSize { x: 16.0, y: 16.0 }, map_type, &TilemapAnchor::Center).extend(1.0)
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
            target.center_in_world(tilemap_size, grid_size, &TilemapTileSize { x: 16.0, y: 16.0 }, map_type, &TilemapAnchor::Center).extend(1.0)
        );
        movement.is_moving = true;
    }
}

fn get_next_position_towards(from: TilePos, to: TilePos, tilemap_size: &TilemapSize) -> Option<TilePos> {
    let dx = (to.x as i32 - from.x as i32).clamp(-1, 1);
    let dy = (to.y as i32 - from.y as i32).clamp(-1, 1);
    
    let new_x = (from.x as i32 + dx) as u32;
    let new_y = (from.y as i32 + dy) as u32;
    
    if new_x < tilemap_size.x && new_y < tilemap_size.y {
        Some(TilePos { x: new_x, y: new_y })
    } else {
        None
    }
}

fn get_next_position_away(from: TilePos, away_from: TilePos, tilemap_size: &TilemapSize) -> Option<TilePos> {
    let dx = (from.x as i32 - away_from.x as i32).clamp(-1, 1);
    let dy = (from.y as i32 - away_from.y as i32).clamp(-1, 1);
    
    let new_x = (from.x as i32 + dx) as u32;
    let new_y = (from.y as i32 + dy) as u32;
    
    if new_x < tilemap_size.x && new_y < tilemap_size.y {
        Some(TilePos { x: new_x, y: new_y })
    } else {
        None
    }
}

fn monster_movement_system(
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
                
                // Update monster position
                if let Some(next_tile) = movement.path.pop_front() {
                    *monster_pos = next_tile;
                }
            } else {
                let move_direction = direction.normalize();
                transform.translation += move_direction * movement.movement_speed * time.delta_secs();
            }
        }
    }
}