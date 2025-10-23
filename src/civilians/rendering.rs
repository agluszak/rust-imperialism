use bevy::picking::prelude::Pickable;
use bevy::prelude::*;

use super::systems::handle_civilian_click;
use super::types::{Civilian, CivilianJob};
use crate::assets::civilian_asset_path;
use crate::map::rendering::{MapVisual, MapVisualFor};
use crate::map::tile_pos::TilePosExt;

const ENGINEER_SIZE: f32 = 64.0; // Match tile size
const ENGINEER_SELECTED_COLOR: Color = Color::srgb(1.0, 0.8, 0.0); // Yellow/gold tint for selected units

/// Create visual sprites for newly added civilian units
/// Uses relationship pattern - sprite automatically despawns when civilian is removed
pub fn render_civilian_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    new_civilians: Query<(Entity, &Civilian), Added<Civilian>>,
) {
    // Only process newly added civilians - relationship handles cleanup automatically
    for (civilian_entity, civilian) in new_civilians.iter() {
        let pos = civilian.position.to_world_pos();

        // Load the appropriate sprite for this civilian type
        let texture: Handle<Image> = asset_server.load(civilian_asset_path(civilian.kind));

        // Tint sprite based on selection (white = normal, yellow = selected)
        let color = if civilian.selected {
            ENGINEER_SELECTED_COLOR
        } else {
            Color::WHITE // No tint for unselected
        };

        info!(
            "Creating visual for {:?} at tile ({}, {}) -> world pos ({}, {})",
            civilian.kind, civilian.position.x, civilian.position.y, pos.x, pos.y
        );

        // Spawn sprite with relationship component - creates bidirectional link
        commands
            .spawn((
                Sprite {
                    image: texture,
                    color,
                    custom_size: Some(Vec2::new(ENGINEER_SIZE, ENGINEER_SIZE)),
                    ..default()
                },
                Transform::from_translation(pos.extend(3.0)), // Above other visuals
                MapVisualFor(civilian_entity),                // Relationship: sprite -> civilian
                Pickable::default(),
            ))
            .observe(handle_civilian_click);

        info!("Spawned civilian visual with relationship-based tracking");
    }
}

/// Update civilian visual colors based on selection, job status, and movement
/// Uses relationship pattern for O(1) sprite lookups
pub fn update_civilian_visual_colors(
    civilians: Query<(Entity, &Civilian, Option<&CivilianJob>, Option<&MapVisual>)>,
    mut visuals: Query<(&mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    // Calculate blink factor for working civilians (oscillates between 0.5 and 1.0)
    let blink_factor = (time.elapsed_secs() * 2.0).sin() * 0.25 + 0.75;

    // Update visuals based on civilian state - O(1) lookup via relationship
    for (_civilian_entity, civilian, job, visual) in civilians.iter() {
        // If civilian has a visual, update it
        if let Some(visual) = visual
            && let Ok((mut sprite, mut transform)) = visuals.get_mut(visual.entity())
        {
            // Determine color based on state priority:
            // 1. Selected (yellow)
            // 2. Working on job (green blink)
            // 3. Moved this turn (desaturated)
            // 4. Default (white)
            let color = if civilian.selected {
                ENGINEER_SELECTED_COLOR
            } else if job.is_some() {
                // Working: blink green
                Color::srgb(0.3 * blink_factor, 1.0 * blink_factor, 0.3 * blink_factor)
            } else if civilian.has_moved {
                // Moved: desaturated (gray)
                Color::srgb(0.6, 0.6, 0.6)
            } else {
                Color::WHITE // Default: no tint
            };
            sprite.color = color;

            // Update position
            let pos = civilian.position.to_world_pos();
            transform.translation = pos.extend(3.0);
        }
    }
}
