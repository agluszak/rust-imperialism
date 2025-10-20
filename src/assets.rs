//! Asset mapping for Imperialism tiles and units
//!
//! Maps game entities to their corresponding PNG image files
//! (converted from original BMP files with transparency)

use crate::civilians::CivilianKind;
use crate::map::tiles::TerrainType;

/// Get the asset path for a terrain type (loads BMP directly)
pub fn terrain_asset_path(terrain: TerrainType) -> &'static str {
    match terrain {
        TerrainType::Grass => "extracted/bitmaps/10000.BMP",
        TerrainType::Water => "extracted/bitmaps/10005.BMP",
        TerrainType::Mountain => "extracted/bitmaps/10003.BMP",
        TerrainType::Hills => "extracted/bitmaps/10002.BMP",
        TerrainType::Forest => "extracted/bitmaps/10001.BMP",
        TerrainType::Desert => "extracted/bitmaps/10006.BMP",
        TerrainType::Swamp => "extracted/bitmaps/10004.BMP",
        TerrainType::Farmland => "extracted/bitmaps/10007.BMP",
    }
}

/// Get the asset path for a civilian unit type (loads BMP directly)
pub fn civilian_asset_path(kind: CivilianKind) -> &'static str {
    match kind {
        CivilianKind::Engineer => "extracted/bitmaps/400.BMP",
        CivilianKind::Farmer => "extracted/bitmaps/401.BMP",
        CivilianKind::Miner => "extracted/bitmaps/402.BMP",
        CivilianKind::Prospector => "extracted/bitmaps/403.BMP",
        CivilianKind::Developer => "extracted/bitmaps/404.BMP",
        CivilianKind::Forester => "extracted/bitmaps/406.BMP",
        CivilianKind::Rancher => "extracted/bitmaps/407.BMP",
        CivilianKind::Driller => "extracted/bitmaps/408.BMP",
    }
}

/// Get the asset path for a depot (loads BMP directly)
pub fn depot_asset_path() -> &'static str {
    "extracted/bitmaps/554.BMP"
}

/// Get the asset path for a port (loads BMP directly)
pub fn port_asset_path() -> &'static str {
    "extracted/bitmaps/557.BMP"
}

/// Get the asset path for a capital city (loads BMP directly)
pub fn capital_asset_path() -> &'static str {
    "extracted/bitmaps/550.BMP"
}

/// Get the asset path for a town/city (loads BMP directly)
/// Uses town_small for now - could be enhanced to show different sizes
pub fn town_asset_path() -> &'static str {
    "extracted/bitmaps/551.BMP"
}
