use bevy::picking::prelude::Pickable;
use bevy::prelude::*;

use super::systems::handle_civilian_click;
use super::types::{Civilian, CivilianJob, CivilianVisual};
use crate::tile_pos::TilePosExt;

const ENGINEER_SIZE: f32 = 64.0; // Match tile size
const ENGINEER_SELECTED_COLOR: Color = Color::srgb(1.0, 0.8, 0.0); // Yellow/gold tint for selected units

/// Create/update visual sprites for civilian units
pub fn render_civilian_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    all_civilians: Query<(Entity, &Civilian)>,
    existing_visuals: Query<(Entity, &CivilianVisual)>,
) {
    // Remove visuals for despawned civilians
    for (visual_entity, civilian_visual) in existing_visuals.iter() {
        if all_civilians.get(civilian_visual.0).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }

    // Create visuals for new civilians
    for (civilian_entity, civilian) in all_civilians.iter() {
        // Check if visual already exists
        let visual_exists = existing_visuals
            .iter()
            .any(|(_, cv)| cv.0 == civilian_entity);

        if !visual_exists {
            let pos = civilian.position.to_world_pos();

            // Load the appropriate sprite for this civilian type
            let texture: Handle<Image> =
                asset_server.load(crate::assets::civilian_asset_path(civilian.kind));

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

            commands
                .spawn((
                    Sprite {
                        image: texture,
                        color,
                        custom_size: Some(Vec2::new(ENGINEER_SIZE, ENGINEER_SIZE)),
                        ..default()
                    },
                    Transform::from_translation(pos.extend(3.0)), // Above other visuals
                    CivilianVisual(civilian_entity),
                    Pickable::default(),
                ))
                .observe(handle_civilian_click);

            info!("Spawned civilian visual with transparency-enabled sprite");
        }
    }
}

/// Update civilian visual colors based on selection, job status, and movement
pub fn update_civilian_visual_colors(
    civilians: Query<(Entity, &Civilian, Option<&CivilianJob>)>,
    mut visuals: Query<(&CivilianVisual, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    // Calculate blink factor for working civilians (oscillates between 0.5 and 1.0)
    let blink_factor = (time.elapsed_secs() * 2.0).sin() * 0.25 + 0.75;

    // Don't use Changed - just update every frame based on current state
    for (civilian_entity, civilian, job) in civilians.iter() {
        for (civilian_visual, mut sprite, mut transform) in visuals.iter_mut() {
            if civilian_visual.0 == civilian_entity {
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
}
