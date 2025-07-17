use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use hexx::Hex;
use std::collections::VecDeque;

#[derive(Component, Debug, Clone)]
pub struct Hero {
    pub name: String,
    pub movement_points: u32,
    pub max_movement_points: u32,
    pub is_selected: bool,
    pub hp: u32,
    pub max_hp: u32,
    pub kills: u32,
}

#[derive(Component, Debug, Clone)]
pub struct HeroMovement {
    pub path: VecDeque<TilePos>,
    pub movement_speed: f32,
    pub is_moving: bool,
    pub target_world_pos: Option<Vec3>,
}

#[derive(Component)]
pub struct HeroSprite;

impl Default for Hero {
    fn default() -> Self {
        Self {
            name: "Hero".to_string(),
            movement_points: 3,
            max_movement_points: 3,
            is_selected: false,
            hp: 10,
            max_hp: 10,
            kills: 0,
        }
    }
}

impl Default for HeroMovement {
    fn default() -> Self {
        Self {
            path: VecDeque::new(),
            movement_speed: 200.0,
            is_moving: false,
            target_world_pos: None,
        }
    }
}

impl Hero {
    pub fn new(name: String, movement_points: u32) -> Self {
        Self {
            name,
            movement_points,
            max_movement_points: movement_points,
            is_selected: false,
            hp: 10,
            max_hp: 10,
            kills: 0,
        }
    }

    pub fn can_move(&self, distance: u32) -> bool {
        self.movement_points >= distance
    }

    pub fn consume_movement(&mut self, distance: u32) {
        self.movement_points = self.movement_points.saturating_sub(distance);
    }

    pub fn refresh_movement(&mut self) {
        self.movement_points = self.max_movement_points;
    }

    pub fn select(&mut self) {
        self.is_selected = true;
    }

    pub fn deselect(&mut self) {
        self.is_selected = false;
    }

    pub fn take_damage(&mut self, damage: u32) {
        self.hp = self.hp.saturating_sub(damage);
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn can_attack(&self) -> bool {
        self.movement_points >= 1
    }

    pub fn attack(&mut self) -> bool {
        if self.can_attack() {
            self.movement_points -= 1;
            true
        } else {
            false
        }
    }

    pub fn add_kill(&mut self) {
        self.kills += 1;
        // Heal after every 3 kills
        if self.kills % 3 == 0 {
            self.hp = self.max_hp;
            println!("Hero healed to full HP after {} kills!", self.kills);
        }
    }
}

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            hero_movement_system,
            hero_selection_visual_system,
        ));
    }
}

fn hero_movement_system(
    time: Res<Time>,
    mut hero_query: Query<(&mut Transform, &mut HeroMovement), With<Hero>>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapTileSize, &TilemapType)>,
) {
    let Ok((tilemap_size, grid_size, tile_size, map_type)) = tilemap_query.single() else {
        return;
    };

    for (mut transform, mut movement) in hero_query.iter_mut() {
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
                
                if !movement.path.is_empty() {
                    movement.path.pop_front();
                    if let Some(next_tile) = movement.path.front() {
                        movement.target_world_pos = Some(
                            next_tile.center_in_world(tilemap_size, grid_size, tile_size, map_type, &TilemapAnchor::Center).extend(1.0)
                        );
                        movement.is_moving = true;
                    }
                }
            } else {
                let move_direction = direction.normalize();
                transform.translation += move_direction * movement.movement_speed * time.delta_secs();
            }
        }
    }
}

fn hero_selection_visual_system(
    mut hero_query: Query<(&Hero, &mut Sprite), (With<HeroSprite>, Changed<Hero>)>,
) {
    for (hero, mut sprite) in hero_query.iter_mut() {
        if hero.is_selected {
            sprite.color = Color::srgb(1.0, 1.0, 0.0); // Yellow when selected
        } else {
            sprite.color = Color::srgb(0.0, 0.0, 1.0); // Blue when not selected
        }
    }
}