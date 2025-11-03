use bevy_ecs_tilemap::prelude::TilePos;

use crate::map::tile_pos::TilePosExt;
use crate::map::tiles::TerrainType;

use crate::economy::technology::{Technologies, Technology};

/// Check if two tiles are adjacent
pub fn are_adjacent(a: TilePos, b: TilePos) -> bool {
    let ha = a.to_hex();
    let hb = b.to_hex();
    ha.distance_to(hb) == 1
}

/// Check if terrain is buildable for rails given technologies
/// Returns (buildable, optional error message)
pub fn can_build_rail_on_terrain(
    terrain: &TerrainType,
    technologies: &Technologies,
) -> (bool, Option<&'static str>) {
    match terrain {
        TerrainType::Water => {
            // Cannot build rails on water
            (false, Some("Cannot build rails on water"))
        }
        TerrainType::Mountain => {
            if technologies.has(Technology::MountainEngineering) {
                (true, None)
            } else {
                (false, Some("Mountain Engineering technology required"))
            }
        }
        TerrainType::Hills => {
            if technologies.has(Technology::HillGrading) {
                (true, None)
            } else {
                (false, Some("Hill Grading technology required"))
            }
        }
        TerrainType::Swamp => {
            if technologies.has(Technology::SwampDrainage) {
                (true, None)
            } else {
                (false, Some("Swamp Drainage technology required"))
            }
        }
        _ => (true, None), // All other terrains are buildable by default
    }
}
