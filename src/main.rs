//! Example of tiles receiving pick events
//! Click on a tile to change its texture.
//! 
//! Camera Controls:
//! - WASD: Move camera
//! - Z: Zoom out (keyboard)
//! - X: Zoom in (keyboard) 
//! - Mouse wheel: Zoom in/out

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_ecs_tilemap::map::HexCoordSystem;
use bevy::ecs::system::ParamSet;

mod helpers;
mod tiles;
mod turn_system;
mod hero;
mod pathfinding;
mod ui;
mod monster;
mod combat;

use crate::helpers::picking::TilemapBackend;
use crate::helpers::camera;
use crate::tiles::{TileType, TileCategory, TerrainType};
use crate::turn_system::{TurnSystem, TurnSystemPlugin};
use crate::hero::{Hero, HeroMovement, HeroSprite, HeroPlugin};
use crate::pathfinding::PathfindingSystem;
use crate::ui::GameUIPlugin;
use crate::monster::{Monster, MonsterPlugin};
use crate::combat::{CombatPlugin, CombatEvent};

/// mostly the same as the `basic` example from `bevy_ecs_tilemap`
fn tilemap_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Print controls to console
    println!("=== Game Controls ===");
    println!("WASD: Move camera");
    println!("Z: Zoom out (keyboard)");
    println!("X: Zoom in (keyboard)");
    println!("Mouse wheel: Zoom in/out");
    println!("Left click: Select hero or move hero");
    println!("Right click: Cycle terrain types");
    println!("Space: End turn");
    println!("=====================");
    
    // Asset by Kenney
    let texture_handle: Handle<Image> = asset_server.load("colored_packed.png");
    let map_size = TilemapSize { x: 20, y: 20 };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            
            // Create different terrain types based on position for variety
            let tile_type = if x < 5 {
                TileType::terrain(TerrainType::Water)
            } else if x > 15 {
                TileType::terrain(TerrainType::Mountain)
            } else if y < 5 {
                TileType::terrain(TerrainType::Forest)
            } else if y > 15 {
                TileType::terrain(TerrainType::Desert)
            } else {
                TileType::terrain(TerrainType::Grass)
            };
            
            let texture_index = tile_type.get_texture_index();
            
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(texture_index),
                        ..default()
                    },
                    tile_type, // Add the tile type component
                ))
                .observe(handle_tile_click)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16., y: 16. };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Hexagon(HexCoordSystem::Row);

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle.clone()),
        tile_size,
        anchor: TilemapAnchor::Center,
        ..Default::default()
    });
    
    // Spawn hero at starting position
    let hero_pos = TilePos { x: 10, y: 10 };
    let hero_world_pos = hero_pos.center_in_world(&map_size, &grid_size, &tile_size, &map_type, &TilemapAnchor::Center);
    
    commands.spawn((
        Hero::new("Player Hero".to_string(), 3),
        HeroMovement::default(),
        hero_pos,
        HeroSprite,
        Sprite {
            color: Color::srgb(0.0, 0.0, 1.0), // Blue color for hero
            custom_size: Some(Vec2::new(12.0, 12.0)),
            ..default()
        },
        Transform::from_translation(hero_world_pos.extend(1.0)),
    ));
}

fn handle_tile_click(
    trigger: Trigger<Pointer<Click>>,
    mut queries: ParamSet<(
        Query<(&mut TileTextureIndex, &mut TileType, &TilePos)>,
        Query<(&TileType, &TilePos)>,
        Query<(&mut Hero, &mut HeroMovement, &mut TilePos), With<Hero>>,
        Query<(Entity, &Monster, &TilePos), With<Monster>>,
    )>,
    tilemap_query: Query<(&TilemapSize, &TileStorage, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
    mut combat_events: EventWriter<CombatEvent>,
    turn_system: Res<TurnSystem>,
) {
    let entity = trigger.target();
    let pointer_button = trigger.event().button;
    
    // Get tilemap info
    let Ok((tilemap_size, _tile_storage, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };
    
    // Get the clicked tile position directly from the entity
    let target_pos = if let Ok((_, tile_pos)) = queries.p1().get(entity) {
        *tile_pos
    } else {
        return;
    };
    
    match pointer_button {
        PointerButton::Primary => {
            // Left click: Hero movement
            if !turn_system.is_player_turn() {
                return;
            }
            
            // Check if clicking on hero (select)
            for (mut hero, _movement, hero_pos) in queries.p2().iter_mut() {
                if *hero_pos == target_pos {
                    // Select/deselect hero
                    if hero.is_selected {
                        hero.deselect();
                    } else {
                        hero.select();
                    }
                    return;
                }
            }
            
            // Get movement cost for the tile
            let movement_cost = if let Ok((tile_type, _)) = queries.p1().get(entity) {
                tile_type.properties.movement_cost as u32
            } else {
                1 // Default cost
            };
            
            // Check if there's a monster at the target position first
            let mut monster_at_target = None;
            {
                let monster_query = queries.p3();
                for (monster_entity, monster, monster_pos) in monster_query.iter() {
                    if *monster_pos == target_pos {
                        monster_at_target = Some((monster_entity, monster.name.clone()));
                        break;
                    }
                }
            }
            
            // Now handle hero actions
            let mut hero_query = queries.p2();
            for (mut hero, mut hero_movement, mut hero_pos) in hero_query.iter_mut() {
                if hero.is_selected {
                    if let Some((_monster_entity, monster_name)) = monster_at_target {
                        // Attack the monster
                        if hero.can_attack() {
                            hero.attack();
                            // We'll rely on the auto-combat system to handle combat when hero moves adjacent
                            println!("Hero attacks {}!", monster_name);
                        } else {
                            println!("Hero doesn't have enough movement points to attack!");
                        }
                    } else {
                        // Move to the tile
                        if hero.can_move(movement_cost) {
                            hero.consume_movement(movement_cost);
                            
                            // Create a simple direct path for now
                            hero_movement.path = vec![target_pos].into();
                            
                            if let Some(first_step) = hero_movement.path.front() {
                                hero_movement.target_world_pos = Some(
                                    first_step.center_in_world(tilemap_size, grid_size, &TilemapTileSize { x: 16.0, y: 16.0 }, map_type, &TilemapAnchor::Center).extend(1.0)
                                );
                                hero_movement.is_moving = true;
                            }
                            
                            // Update hero position immediately
                            *hero_pos = target_pos;
                            
                            println!("Hero moving to {:?}, cost: {}, remaining movement: {}", 
                                    target_pos, movement_cost, hero.movement_points);
                        } else {
                            println!("Not enough movement points! Need {}, have {}", 
                                    movement_cost, hero.movement_points);
                        }
                    }
                    break;
                }
            }
        },
        PointerButton::Secondary => {
            // Right click: Cycle through terrain types
            if let Ok((mut texture_index, mut tile_type, _)) = queries.p0().get_mut(entity) {
                let new_terrain = match &tile_type.category {
                    TileCategory::Terrain(TerrainType::Grass) => TerrainType::Water,
                    TileCategory::Terrain(TerrainType::Water) => TerrainType::Mountain,
                    TileCategory::Terrain(TerrainType::Mountain) => TerrainType::Desert,
                    TileCategory::Terrain(TerrainType::Desert) => TerrainType::Forest,
                    TileCategory::Terrain(TerrainType::Forest) => TerrainType::Snow,
                    TileCategory::Terrain(TerrainType::Snow) => TerrainType::Grass,
                    _ => TerrainType::Grass, // Default fallback
                };
                
                *tile_type = TileType::terrain(new_terrain);
                texture_index.0 = tile_type.get_texture_index();
            }
        },
        _ => {}
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            TilemapPlugin,
            // The additional backend to check events against the tiles
            TilemapBackend,
            // Game systems
            TurnSystemPlugin,
            HeroPlugin,
            GameUIPlugin,
            MonsterPlugin,
            CombatPlugin,
        ))
        .add_systems(
            Startup,
            (tilemap_startup, |mut commands: Commands| {
                commands.spawn((
                    Camera2d,
                    Projection::Orthographic(OrthographicProjection {
                        scale: 0.5,
                        ..OrthographicProjection::default_2d()
                    }),
                ));
            }),
        )
        .add_systems(Update, (
            camera::movement,
            hero_turn_refresh,
            update_hero_position,
        ))
        .run();
}

// System to refresh hero movement points at start of turn
fn hero_turn_refresh(
    mut hero_query: Query<&mut Hero>,
    turn_system: Res<TurnSystem>,
) {
    if turn_system.is_changed() && turn_system.is_player_turn() {
        for mut hero in hero_query.iter_mut() {
            hero.refresh_movement();
        }
    }
}

// System to update hero tile position when movement is complete
fn update_hero_position(
    mut hero_query: Query<(&mut TilePos, &HeroMovement, &Transform), With<Hero>>,
    tilemap_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType), With<TilemapGridSize>>,
) {
    let Ok((tilemap_size, grid_size, map_type)) = tilemap_query.single() else {
        return;
    };
    
    for (mut tile_pos, movement, transform) in hero_query.iter_mut() {
        if !movement.is_moving && movement.path.is_empty() {
            // Update tile position based on world position
            if let Some(new_pos) = TilePos::from_world_pos(
                &transform.translation.xy(),
                tilemap_size,
                grid_size,
                &TilemapTileSize { x: 16.0, y: 16.0 },
                map_type,
                &TilemapAnchor::Center,
            ) {
                if *tile_pos != new_pos {
                    *tile_pos = new_pos;
                }
            }
        }
    }
}
