use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::map::tile_pos::TilePosExt;
use crate::resources::{DevelopmentLevel, TileResource};
use crate::ui::components::MapTilemap;

/// Marker component indicating a tile has a visible improvement building
#[derive(Component, Debug, Clone, Copy)]
pub struct TileImprovement {
    pub development_level: DevelopmentLevel,
}

/// Plugin to render improvement markers on tiles
pub struct ImprovementRenderingPlugin;

impl Plugin for ImprovementRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (render_improvement_markers, update_improvement_markers),
        );
    }
}

const IMPROVEMENT_SIZE: f32 = 16.0; // Small indicator
const IMPROVEMENT_OFFSET_Y: f32 = 20.0; // Offset from tile center

/// Render visual markers for newly improved tiles
fn render_improvement_markers(
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
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(IMPROVEMENT_SIZE, IMPROVEMENT_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(1.5)), // Above terrain, below cities
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapTilemap,                        // Marker for visibility control
            ImprovementMarkerFor(tile_entity), // Track which tile this belongs to
        ));
    }
}

/// Component linking improvement marker to its tile
#[derive(Component)]
struct ImprovementMarkerFor(Entity);

/// Update improvement markers when development level changes
fn update_improvement_markers(
    mut commands: Commands,
    changed_tiles: Query<
        (Entity, &TilePos, &TileImprovement, &TileResource),
        Changed<TileImprovement>,
    >,
    markers: Query<(Entity, &ImprovementMarkerFor)>,
) {
    for (tile_entity, tile_pos, improvement, _resource) in changed_tiles.iter() {
        // Find and remove old marker
        for (marker_entity, marker_for) in markers.iter() {
            if marker_for.0 == tile_entity {
                commands.entity(marker_entity).despawn();
                break;
            }
        }

        // Create new marker with updated color
        let mut pos = tile_pos.to_world_pos();
        pos.y += IMPROVEMENT_OFFSET_Y;

        let color = match improvement.development_level {
            DevelopmentLevel::Lv1 => Color::srgb(0.6, 0.8, 0.4),
            DevelopmentLevel::Lv2 => Color::srgb(0.4, 0.9, 0.2),
            DevelopmentLevel::Lv3 => Color::srgb(1.0, 0.85, 0.0),
            _ => Color::WHITE,
        };

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
}
