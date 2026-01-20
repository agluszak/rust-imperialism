use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::NationColor;
use crate::economy::production::ConnectedProduction;
use crate::map::tile_pos::TilePosExt;
use crate::resources::{DevelopmentLevel, TileResource};
use crate::ui::components::MapTilemap;

/// Marker component indicating a tile has a visible improvement building
#[derive(Component, Debug, Clone, Copy)]
pub struct TileImprovement {
    pub development_level: DevelopmentLevel,
}

/// Relationship component linking the spawned marker sprite back to the tile entity.
/// The [`TileImprovementMarker`] target component is automatically attached to the tile,
/// providing O(1) lookups when updating or despawning the marker.
#[derive(Component)]
#[relationship(relationship_target = TileImprovementMarker)]
pub struct ImprovementMarkerFor(pub Entity);

/// Reverse relationship component automatically attached to improved tiles.
/// Stores the entity of the spawned sprite so we can update or despawn it efficiently.
#[derive(Component)]
#[relationship_target(relationship = ImprovementMarkerFor)]
pub struct TileImprovementMarker(Entity);

impl TileImprovementMarker {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

/// Runtime toggle for connectivity indicator overlay (C key)
#[derive(Resource, Default)]
pub struct ConnectivityOverlaySettings {
    pub enabled: bool,
}

/// Marker component for connectivity indicator sprites
#[derive(Component)]
pub struct ConnectivityIndicator;

const IMPROVEMENT_SIZE: f32 = 16.0; // Small indicator
const IMPROVEMENT_OFFSET_Y: f32 = 20.0; // Offset from tile center
const CONNECTIVITY_SIZE: f32 = 10.0; // Small connectivity indicator
const CONNECTIVITY_OFFSET_X: f32 = -12.0; // Offset to the left of tile center
const CONNECTIVITY_OFFSET_Y: f32 = 20.0; // Same vertical level as improvement marker

/// Render visual markers for newly improved tiles
pub fn render_improvement_markers(
    mut commands: Commands,
    new_improvements: Query<
        (Entity, &TilePos, &TileImprovement, &TileResource),
        Added<TileImprovement>,
    >,
) {
    for (tile_entity, tile_pos, improvement, resource) in new_improvements.iter() {
        let mut pos = tile_pos.to_world_pos();
        pos.y += IMPROVEMENT_OFFSET_Y; // Offset upward from tile center

        // Color based on development level
        let color = match improvement.development_level {
            DevelopmentLevel::Lv1 => Color::srgb(0.6, 0.8, 0.4), // Light green
            DevelopmentLevel::Lv2 => Color::srgb(0.4, 0.9, 0.2), // Bright green
            DevelopmentLevel::Lv3 => Color::srgb(1.0, 0.85, 0.0), // Gold
            _ => Color::WHITE,
        };

        info!(
            "Creating improvement marker for {:?} at ({}, {}) - Level {:?}",
            resource.resource_type, tile_pos.x, tile_pos.y, improvement.development_level
        );

        // Spawn a simple colored square as improvement marker
        spawn_marker(&mut commands, tile_entity, pos, color);
    }
}

/// Update improvement markers when development level changes
pub fn update_improvement_markers(
    changed_tiles: Query<
        (
            Entity,
            &TilePos,
            &TileImprovement,
            &TileResource,
            Option<&TileImprovementMarker>,
        ),
        Changed<TileImprovement>,
    >,
    mut marker_visuals: Query<(&mut Sprite, &mut Transform), With<ImprovementMarkerFor>>,
    mut commands: Commands,
) {
    for (tile_entity, tile_pos, improvement, _resource, maybe_marker) in changed_tiles.iter() {
        let mut pos = tile_pos.to_world_pos();
        pos.y += IMPROVEMENT_OFFSET_Y;

        let color = match improvement.development_level {
            DevelopmentLevel::Lv1 => Color::srgb(0.6, 0.8, 0.4),
            DevelopmentLevel::Lv2 => Color::srgb(0.4, 0.9, 0.2),
            DevelopmentLevel::Lv3 => Color::srgb(1.0, 0.85, 0.0),
            _ => Color::WHITE,
        };

        if let Some(marker) = maybe_marker {
            if let Ok((mut sprite, mut transform)) = marker_visuals.get_mut(marker.entity()) {
                sprite.color = color;
                transform.translation = pos.extend(1.5);
            } else {
                warn!(
                    "Tile {:?} lost its improvement marker entity; respawning",
                    tile_entity
                );
                spawn_marker(&mut commands, tile_entity, pos, color);
            }
        } else {
            spawn_marker(&mut commands, tile_entity, pos, color);
        }
    }
}

/// Remove improvement markers automatically when the tile loses its improvement component.
pub fn cleanup_removed_improvement_markers(
    mut removed_improvements: RemovedComponents<TileImprovement>,
    markers: Query<&TileImprovementMarker>,
    mut commands: Commands,
) {
    for tile_entity in removed_improvements.read() {
        if let Ok(marker) = markers.get(tile_entity) {
            commands.entity(marker.entity()).despawn();
        }
    }
}

fn spawn_marker(commands: &mut Commands, tile_entity: Entity, pos: Vec2, color: Color) {
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(IMPROVEMENT_SIZE, IMPROVEMENT_SIZE)),
            ..default()
        },
        Transform::from_translation(pos.extend(1.5)),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        MapTilemap,
        ImprovementMarkerFor(tile_entity),
    ));
}

/// Toggle connectivity overlay with C key
pub fn toggle_connectivity_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ConnectivityOverlaySettings>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        settings.enabled = !settings.enabled;
        info!(
            "Connectivity overlay: {}",
            if settings.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// Update connectivity overlay - spawn/despawn/update indicators based on settings
///
/// Shows colored indicators for ALL nations:
/// - Square (■) = connected to transport network
/// - Cross (X) = not connected (improved tiles only)
///
/// Color matches the nation's border color.
pub fn update_connectivity_overlay(
    mut commands: Commands,
    settings: Res<ConnectivityOverlaySettings>,
    connected_production: Res<ConnectedProduction>,
    nations: Query<(Entity, &NationColor)>,
    improved_tiles: Query<(&TilePos, &TileResource), With<TileImprovement>>,
    existing_indicators: Query<Entity, With<ConnectivityIndicator>>,
) {
    // If disabled, despawn all indicators
    if !settings.enabled {
        for entity in existing_indicators.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Rebuild indicators when settings or connected_production changes
    if settings.is_changed() || connected_production.is_changed() {
        // Despawn existing indicators
        for entity in existing_indicators.iter() {
            commands.entity(entity).despawn();
        }

        // Build a map of nation entity -> color
        let nation_colors: std::collections::HashMap<Entity, Color> = nations
            .iter()
            .map(|(entity, color)| (entity, color.0))
            .collect();

        // Collect all connected tile positions (across all nations)
        let all_connected_positions: std::collections::HashSet<TilePos> = connected_production
            .tiles
            .iter()
            .map(|t| t.tile_pos)
            .collect();

        // Show SQUARE indicators for all connected tiles (all nations)
        for tile in &connected_production.tiles {
            let color = nation_colors
                .get(&tile.owner)
                .copied()
                .unwrap_or(Color::WHITE);

            let mut pos = tile.tile_pos.to_world_pos();
            pos.x += CONNECTIVITY_OFFSET_X;
            pos.y += CONNECTIVITY_OFFSET_Y;

            // Spawn a filled square for connected tiles
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(CONNECTIVITY_SIZE, CONNECTIVITY_SIZE)),
                    ..default()
                },
                Transform::from_translation(pos.extend(1.6)),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                MapTilemap,
                ConnectivityIndicator,
            ));
        }

        // Show CROSS (X) indicators for improved tiles that are NOT connected
        // Use a dark red/gray color for unconnected since we don't know the owner
        let unconnected_color = Color::srgb(0.6, 0.2, 0.2);
        let cross_thickness = 3.0;
        let cross_length = CONNECTIVITY_SIZE;

        for (tile_pos, _resource) in improved_tiles.iter() {
            if !all_connected_positions.contains(tile_pos) {
                let mut pos = tile_pos.to_world_pos();
                pos.x += CONNECTIVITY_OFFSET_X;
                pos.y += CONNECTIVITY_OFFSET_Y;

                // Spawn two rectangles rotated 45 degrees to form an X
                // First diagonal (\)
                commands.spawn((
                    Sprite {
                        color: unconnected_color,
                        custom_size: Some(Vec2::new(cross_length, cross_thickness)),
                        ..default()
                    },
                    Transform::from_translation(pos.extend(1.6))
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    MapTilemap,
                    ConnectivityIndicator,
                ));

                // Second diagonal (/)
                commands.spawn((
                    Sprite {
                        color: unconnected_color,
                        custom_size: Some(Vec2::new(cross_length, cross_thickness)),
                        ..default()
                    },
                    Transform::from_translation(pos.extend(1.6))
                        .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4)),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    MapTilemap,
                    ConnectivityIndicator,
                ));
            }
        }

        if settings.is_changed() && settings.enabled {
            let unconnected_count = improved_tiles
                .iter()
                .filter(|(pos, _)| !all_connected_positions.contains(*pos))
                .count();
            info!(
                "Connectivity overlay: {} connected, {} unconnected improvements",
                connected_production.tiles.len(),
                unconnected_count
            );
        }
    }
}
