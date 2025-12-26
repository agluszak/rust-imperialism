use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::civilians::types::ProspectingKnowledge;
use crate::economy::nation::PlayerNation;
use crate::map::prospecting::{ProspectedEmpty, ProspectedMineral};
use crate::map::tile_pos::TilePosExt;
use crate::resources::ResourceType;
use crate::ui::components::MapTilemap;

/// Plugin to render prospecting result markers
pub struct ProspectingMarkersPlugin;

impl Plugin for ProspectingMarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_prospected_empty_markers,
                render_prospected_mineral_markers,
            ),
        );
    }
}

const MARKER_SIZE: f32 = 20.0;
const MARKER_OFFSET_Y: f32 = 15.0; // Offset from tile center

/// Component linking marker to its tile
#[derive(Component)]
pub struct ProspectingMarkerFor(pub Entity);

/// Render red cross markers for tiles prospected with no result
/// Only renders markers for tiles prospected by the player's nation
pub fn render_prospected_empty_markers(
    mut commands: Commands,
    new_empty: Query<(Entity, &TilePos), Added<ProspectedEmpty>>,
    player_nation: Option<Res<PlayerNation>>,
    prospecting_knowledge: Res<ProspectingKnowledge>,
) {
    let Some(player_nation) = player_nation else {
        return;
    };
    let player_entity = player_nation.entity();

    for (tile_entity, tile_pos) in new_empty.iter() {
        // Only render markers for tiles prospected by the player
        if !prospecting_knowledge.is_discovered_by(tile_entity, player_entity) {
            continue;
        }

        let mut pos = tile_pos.to_world_pos();
        pos.y += MARKER_OFFSET_Y;

        info!(
            "Creating 'empty' marker at ({}, {}) for player's nation",
            tile_pos.x, tile_pos.y
        );

        // Spawn a red 'X' using two rotated rectangles
        commands.spawn((
            Sprite {
                color: Color::srgb(0.9, 0.1, 0.1), // Bright red
                custom_size: Some(Vec2::new(MARKER_SIZE * 0.8, MARKER_SIZE * 0.15)),
                ..default()
            },
            Transform::from_translation(pos.extend(1.5))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::PI / 4.0)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapTilemap,
            ProspectingMarkerFor(tile_entity),
        ));

        commands.spawn((
            Sprite {
                color: Color::srgb(0.9, 0.1, 0.1), // Bright red
                custom_size: Some(Vec2::new(MARKER_SIZE * 0.8, MARKER_SIZE * 0.15)),
                ..default()
            },
            Transform::from_translation(pos.extend(1.5))
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::PI / 4.0)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapTilemap,
            ProspectingMarkerFor(tile_entity),
        ));
    }
}

/// Render colored square markers for discovered minerals
/// Only renders markers for tiles prospected by the player's nation
pub fn render_prospected_mineral_markers(
    mut commands: Commands,
    new_minerals: Query<(Entity, &TilePos, &ProspectedMineral), Added<ProspectedMineral>>,
    player_nation: Option<Res<PlayerNation>>,
    prospecting_knowledge: Res<ProspectingKnowledge>,
) {
    let Some(player_nation) = player_nation else {
        return;
    };
    let player_entity = player_nation.entity();

    for (tile_entity, tile_pos, mineral) in new_minerals.iter() {
        // Only render markers for tiles prospected by the player
        if !prospecting_knowledge.is_discovered_by(tile_entity, player_entity) {
            continue;
        }

        let mut pos = tile_pos.to_world_pos();
        pos.y += MARKER_OFFSET_Y;

        // Choose color based on resource type
        let color = match mineral.resource_type {
            ResourceType::Coal => Color::srgb(0.1, 0.1, 0.1), // Black
            ResourceType::Iron => Color::srgb(0.4, 0.3, 0.25), // Brown
            ResourceType::Gold => Color::srgb(1.0, 0.84, 0.0), // Gold
            ResourceType::Gems => Color::srgb(0.2, 0.4, 0.9), // Blue
            ResourceType::Oil => Color::srgb(0.05, 0.05, 0.05), // Black (darker than coal)
            _ => Color::WHITE,                                // Shouldn't happen for minerals
        };

        info!(
            "Creating {:?} marker at ({}, {}) for player's nation",
            mineral.resource_type, tile_pos.x, tile_pos.y
        );

        // Spawn colored square
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(MARKER_SIZE, MARKER_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(1.5)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapTilemap,
            ProspectingMarkerFor(tile_entity),
        ));
    }
}
