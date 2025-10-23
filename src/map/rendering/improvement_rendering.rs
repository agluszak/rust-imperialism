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

/// Relationship component linking the spawned marker sprite back to the tile entity.
/// The [`TileImprovementMarker`] target component is automatically attached to the tile,
/// providing O(1) lookups when updating or despawning the marker.
#[derive(Component)]
#[relationship(relationship_target = TileImprovementMarker)]
struct ImprovementMarkerFor(Entity);

/// Reverse relationship component automatically attached to improved tiles.
/// Stores the entity of the spawned sprite so we can update or despawn it efficiently.
#[derive(Component)]
#[relationship_target(relationship = ImprovementMarkerFor)]
struct TileImprovementMarker(Entity);

impl TileImprovementMarker {
    fn entity(&self) -> Entity {
        self.0
    }
}

/// Plugin to render improvement markers on tiles
pub struct ImprovementRenderingPlugin;

impl Plugin for ImprovementRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_improvement_markers,
                update_improvement_markers,
                cleanup_removed_improvement_markers,
            ),
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
        spawn_marker(&mut commands, tile_entity, pos, color);
    }
}

/// Update improvement markers when development level changes
fn update_improvement_markers(
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
fn cleanup_removed_improvement_markers(
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
