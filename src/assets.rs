//! Asset mapping for Imperialism tiles and units
//!
//! Maps game entities to their corresponding PNG image files
//! (converted from original BMP files with transparency)

use crate::civilians::CivilianKind;
use crate::tiles::TerrainType;

/// Get the asset path for a terrain type (loads BMP directly)
pub fn terrain_asset_path(terrain: TerrainType) -> &'static str {
    match terrain {
        TerrainType::Grass => "glop/pictuniv.gob_2_10000.BMP.bmp",
        TerrainType::Water => "glop/pictuniv.gob_2_10005.BMP.bmp",
        TerrainType::Mountain => "glop/pictuniv.gob_2_10003.BMP.bmp",
        TerrainType::Hills => "glop/pictuniv.gob_2_10002.BMP.bmp",
        TerrainType::Forest => "glop/pictuniv.gob_2_10001.BMP.bmp",
        TerrainType::Desert => "glop/pictuniv.gob_2_10006.BMP.bmp",
        TerrainType::Swamp => "glop/pictuniv.gob_2_10004.BMP.bmp",
    }
}

/// Get the asset path for a civilian unit type (loads BMP directly)
pub fn civilian_asset_path(kind: CivilianKind) -> &'static str {
    match kind {
        CivilianKind::Engineer => "glop/pictuniv.gob_2_400.BMP.bmp",
        CivilianKind::Farmer => "glop/pictuniv.gob_2_401.BMP.bmp",
        CivilianKind::Miner => "glop/pictuniv.gob_2_402.BMP.bmp",
        CivilianKind::Prospector => "glop/pictuniv.gob_2_403.BMP.bmp",
        CivilianKind::Developer => "glop/pictuniv.gob_2_404.BMP.bmp",
        CivilianKind::Forester => "glop/pictuniv.gob_2_406.BMP.bmp",
        CivilianKind::Rancher => "glop/pictuniv.gob_2_407.BMP.bmp",
        CivilianKind::Driller => "glop/pictuniv.gob_2_408.BMP.bmp",
    }
}

/// Get the asset path for a depot (loads BMP directly)
pub fn depot_asset_path() -> &'static str {
    "glop/pictuniv.gob_2_554.BMP.bmp"
}

/// Get the asset path for a port (loads BMP directly)
pub fn port_asset_path() -> &'static str {
    "glop/pictuniv.gob_2_557.BMP.bmp"
}

/// Get the asset path for a capital city (loads BMP directly)
pub fn capital_asset_path() -> &'static str {
    "glop/pictuniv.gob_2_550.BMP.bmp"
}

/// Get the asset path for a town/city (loads BMP directly)
/// Uses town_small for now - could be enhanced to show different sizes
pub fn town_asset_path() -> &'static str {
    "glop/pictuniv.gob_2_551.BMP.bmp"
}
