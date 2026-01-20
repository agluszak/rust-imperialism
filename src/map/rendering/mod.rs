use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::ui::components::MapTilemap;

// Rendering modules for map elements
pub mod border_rendering;
pub mod city_rendering;
pub mod improvement_rendering;
pub mod map_visual;
pub mod prospecting_markers;
pub mod terrain_atlas;
pub mod transport_debug;
pub mod transport_rendering;

// Re-exports for convenience
pub use border_rendering::*;
pub use city_rendering::*;
pub use improvement_rendering::*;
pub use map_visual::*;
pub use prospecting_markers::*;
pub use terrain_atlas::*;
pub use transport_debug::*;
pub use transport_rendering::*;

/// Plugin that handles map rendering setup (terrain atlas + tilemap visuals).
pub struct MapRenderingPlugin;

impl Plugin for MapRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, terrain_atlas::start_terrain_atlas_loading)
            .add_systems(
                Update,
                (
                    terrain_atlas::build_terrain_atlas_when_ready,
                    attach_tilemap_rendering,
                ),
            );
    }
}

fn attach_tilemap_rendering(
    mut commands: Commands,
    atlas: Option<Res<terrain_atlas::TerrainAtlas>>,
    tilemaps: Query<
        (
            Entity,
            &TilemapGridSize,
            &TilemapType,
            &TilemapSize,
            &TilemapTileSize,
            &TileStorage,
            &TilemapAnchor,
        ),
        Without<TilemapTexture>,
    >,
) {
    let Some(atlas) = atlas else {
        return;
    };
    if !atlas.ready {
        return;
    }

    for (entity, grid_size, map_type, size, tile_size, storage, anchor) in tilemaps.iter() {
        commands.entity(entity).insert((
            TilemapBundle {
                grid_size: *grid_size,
                map_type: *map_type,
                size: *size,
                storage: storage.clone(),
                texture: TilemapTexture::Single(atlas.texture.clone()),
                tile_size: *tile_size,
                anchor: *anchor,
                ..Default::default()
            },
            MapTilemap,
        ));
    }
}
