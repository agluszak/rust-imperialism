use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::assets::{capital_asset_path, town_asset_path};
use crate::province::City;
use crate::tile_pos::TilePosExt;

/// Plugin to render city sprites on the map
pub struct CityRenderingPlugin;

impl Plugin for CityRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (render_city_visuals, update_city_visual_positions));
    }
}

/// Visual marker for city sprites
#[derive(Component)]
pub struct CityVisual(pub Entity); // Points to the City entity

const CITY_SIZE: f32 = 64.0; // Match tile size

/// Create visual sprites for cities
fn render_city_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cities: Query<(Entity, &City, &TilePos), Added<City>>,
) {
    for (city_entity, city, tile_pos) in cities.iter() {
        let pos = tile_pos.to_world_pos();

        // Load the appropriate sprite based on whether it's a capital
        let texture: Handle<Image> = if city.is_capital {
            asset_server.load(capital_asset_path())
        } else {
            asset_server.load(town_asset_path())
        };

        info!(
            "Creating {} visual at tile ({}, {}) -> world pos ({}, {})",
            if city.is_capital { "capital" } else { "city" },
            tile_pos.x,
            tile_pos.y,
            pos.x,
            pos.y
        );

        commands.spawn((
            Sprite {
                image: texture,
                color: Color::WHITE,
                custom_size: Some(Vec2::new(CITY_SIZE, CITY_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(2.0)), // Below civilians (z=3), above terrain
            CityVisual(city_entity),
        ));
    }
}

/// Update city visual positions when their TilePos changes
fn update_city_visual_positions(
    cities: Query<(Entity, &TilePos), (With<City>, Changed<TilePos>)>,
    mut visuals: Query<(&CityVisual, &mut Transform)>,
) {
    for (city_entity, tile_pos) in cities.iter() {
        for (city_visual, mut transform) in visuals.iter_mut() {
            if city_visual.0 == city_entity {
                let pos = tile_pos.to_world_pos();
                transform.translation = pos.extend(2.0);
            }
        }
    }
}
