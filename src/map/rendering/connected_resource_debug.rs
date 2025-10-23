use bevy::prelude::*;
use bevy::sprite::Text2d;

use crate::economy::production::{
    ConnectedProduction, ConnectedTileSource, calculate_connected_production,
};
use crate::map::tile_pos::TilePosExt;
use crate::ui::components::MapTilemap;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

/// Runtime toggle for the connected resource debug overlay.
#[derive(Resource, Default)]
pub struct ConnectedResourceDebugSettings {
    pub enabled: bool,
}

#[derive(Resource)]
struct ConnectedResourceDebugFont(Handle<Font>);

impl FromWorld for ConnectedResourceDebugFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(asset_server.load("fonts/FiraSans-Bold.ttf"))
    }
}

/// Marker on 2D text entities spawned for the overlay.
#[derive(Component)]
struct ConnectedResourceDebugLabel;

/// Plugin that renders a simple text overlay highlighting connected resources.
pub struct ConnectedResourceDebugPlugin;

impl Plugin for ConnectedResourceDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectedResourceDebugSettings>()
            .init_resource::<ConnectedResourceDebugFont>()
            .add_systems(
                Update,
                (
                    toggle_connected_resource_debug,
                    update_connected_resource_debug_labels.after(calculate_connected_production),
                )
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameMode::Map)),
            );
    }
}

fn toggle_connected_resource_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ConnectedResourceDebugSettings>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        settings.enabled = !settings.enabled;
        info!(
            "Connected resource debug overlay: {}",
            if settings.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

fn update_connected_resource_debug_labels(
    mut commands: Commands,
    settings: Res<ConnectedResourceDebugSettings>,
    connected_production: Res<ConnectedProduction>,
    font: Res<ConnectedResourceDebugFont>,
    existing: Query<Entity, With<ConnectedResourceDebugLabel>>,
) {
    if !settings.enabled {
        if !existing.is_empty() {
            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    if !settings.is_changed() && !connected_production.is_changed() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    for tile in connected_production.tiles.iter() {
        let world_pos = tile.tile_pos.to_world_pos();
        let color = match tile.source {
            ConnectedTileSource::Improvement => Color::srgb(0.2, 0.9, 0.2),
            ConnectedTileSource::Port => Color::srgb(0.3, 0.6, 1.0),
            ConnectedTileSource::Baseline => Color::srgb(0.95, 0.85, 0.2),
        };

        let label = format!(
            "{:?} {}{}",
            tile.resource_type,
            tile.output,
            tile.source.marker()
        );

        commands.spawn((
            Text2d::new(label),
            TextFont {
                font: font.0.clone(),
                font_size: 32.0,
                ..default()
            },
            TextColor(color),
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 4.5))
                .with_scale(Vec3::splat(0.45)),
            ConnectedResourceDebugLabel,
            MapTilemap,
        ));
    }
}
