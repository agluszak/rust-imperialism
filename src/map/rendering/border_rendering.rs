use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TileStorage, TilemapSize};
use std::collections::HashMap;

use crate::economy::NationColor;
use crate::map::province::{Province, ProvinceId, TileProvince};
use crate::map::tile_pos::{HexExt, TilePosExt};

/// Marker component for border line entities
#[derive(Component)]
pub struct BorderLine;

/// Cache for border geometry lines to avoid recalculating every frame
#[derive(Resource, Default)]
pub struct BorderGeometry {
    pub lines: Vec<(Vec2, Vec2, Color)>,
}

/// Render borders between provinces and nations
/// Optimized with change detection and province ownership caching
pub fn render_borders(
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    provinces_changed: Query<Entity, Changed<Province>>,
    mut removed_provinces: RemovedComponents<Province>,
    nations: Query<&NationColor>,
    mut border_geometry: ResMut<BorderGeometry>,
    mut gizmos: Gizmos,
) {
    // Only recalculate if provinces have changed (ownership changes, etc.) or cache is empty
    // Also check if any provinces were removed
    let needs_update = !provinces_changed.is_empty()
        || removed_provinces.read().next().is_some()
        || border_geometry.lines.is_empty();

    if needs_update {
        // Clear old borders
        border_geometry.lines.clear();

        let Some((tile_storage, map_size)) = tile_storage_query.iter().next() else {
            return;
        };

        // Build province ownership lookup map once to avoid O(n²) lookups
        let province_owners: HashMap<ProvinceId, Option<Entity>> =
            provinces.iter().map(|p| (p.id, p.owner)).collect();

        // Check each tile and its neighbors to find borders
        for province in provinces.iter() {
            for &tile_pos in &province.tiles {
                if let Some(tile_entity) = tile_storage.get(&tile_pos)
                    && let Ok(tile_prov) = tile_provinces.get(tile_entity)
                {
                    let tile_hex = tile_pos.to_hex();

                    // Check all 6 neighbors
                    for neighbor_hex in tile_hex.all_neighbors() {
                        if let Some(neighbor_pos) = neighbor_hex.to_tile_pos() {
                            // Bounds check
                            if neighbor_pos.x >= map_size.x || neighbor_pos.y >= map_size.y {
                                continue;
                            }
                            if let Some(neighbor_entity) = tile_storage.get(&neighbor_pos)
                                && let Ok(neighbor_prov) = tile_provinces.get(neighbor_entity)
                            {
                                // Found a border between tiles
                                if tile_prov.province_id != neighbor_prov.province_id {
                                    // Use cached province ownership lookup - O(1) instead of O(n)
                                    let tile_owner = province_owners
                                        .get(&tile_prov.province_id)
                                        .copied()
                                        .flatten();
                                    let neighbor_owner = province_owners
                                        .get(&neighbor_prov.province_id)
                                        .copied()
                                        .flatten();

                                    // Check if it's an international border
                                    let is_international = tile_owner != neighbor_owner;

                                    // Calculate edge position between the two tiles
                                    let start_world = tile_pos.to_world_pos();
                                    let end_world = neighbor_pos.to_world_pos();
                                    let edge_center = Vec2::new(
                                        (start_world.x + end_world.x) / 2.0,
                                        (start_world.y + end_world.y) / 2.0,
                                    );

                                    // Calculate perpendicular for the border line
                                    let direction = (end_world - start_world).normalize();
                                    let perpendicular = Vec2::new(-direction.y, direction.x);
                                    let half_edge_length = 20.0; // Approximately half hex side

                                    let line_start = edge_center - perpendicular * half_edge_length;
                                    let line_end = edge_center + perpendicular * half_edge_length;

                                    // Store the border lines in cache
                                    if is_international {
                                        // International border: draw both nation colors
                                        // Get both nations' colors
                                        let tile_color = tile_owner
                                            .and_then(|owner| nations.get(owner).ok())
                                            .map(|nc| nc.0)
                                            .unwrap_or(Color::WHITE);
                                        let neighbor_color = neighbor_owner
                                            .and_then(|owner| nations.get(owner).ok())
                                            .map(|nc| nc.0)
                                            .unwrap_or(Color::WHITE);

                                        // Draw border closer to each nation
                                        // Tile nation's side (offset towards tile)
                                        let tile_offset = direction * 2.5;
                                        border_geometry.lines.push((
                                            line_start + tile_offset,
                                            line_end + tile_offset,
                                            tile_color,
                                        ));
                                        let tile_offset2 = direction * 1.5;
                                        border_geometry.lines.push((
                                            line_start + tile_offset2,
                                            line_end + tile_offset2,
                                            tile_color,
                                        ));

                                        // Neighbor nation's side (offset towards neighbor)
                                        let neighbor_offset = direction * -2.5;
                                        border_geometry.lines.push((
                                            line_start + neighbor_offset,
                                            line_end + neighbor_offset,
                                            neighbor_color,
                                        ));
                                        let neighbor_offset2 = direction * -1.5;
                                        border_geometry.lines.push((
                                            line_start + neighbor_offset2,
                                            line_end + neighbor_offset2,
                                            neighbor_color,
                                        ));

                                        // Center line for definition
                                        border_geometry.lines.push((
                                            line_start,
                                            line_end,
                                            Color::srgb(0.3, 0.3, 0.3),
                                        ));
                                    } else {
                                        // Thin black provincial border
                                        border_geometry.lines.push((
                                            line_start,
                                            line_end,
                                            Color::BLACK
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw all lines from cache
    for (start, end, color) in &border_geometry.lines {
        gizmos.line_2d(*start, *end, *color);
    }
}
