use bevy::prelude::*;

/// Essential terrain types for gameplay
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum TerrainType {
    Grass,    // Plains - basic terrain, good movement
    Water,    // Ocean/rivers - impassable without ships
    Mountain, // High ground - defensive bonus, slow movement
    Hills,    // Rolling hills - moderate terrain
    Forest,   // Dense vegetation - cover bonus, moderate movement cost
    Desert,   // Harsh terrain - movement penalty, low resources
    Swamp,    // Wetlands - difficult terrain
}

/// Predefined tile types with their indices in the terrain_atlas.png
/// These indices correspond to the order in mapping.csv for terrain types
pub struct TileIndex;

impl TileIndex {
    // Terrain tile indices in the atlas (4x4 grid, 64x64 tiles)
    pub const GRASS: u32 = 0; // pictuniv.gob_2_10000
    pub const FOREST: u32 = 1; // pictuniv.gob_2_10001 (woods)
    pub const HILLS: u32 = 2; // pictuniv.gob_2_10002
    pub const MOUNTAIN: u32 = 3; // pictuniv.gob_2_10003
    pub const SWAMP: u32 = 4; // pictuniv.gob_2_10004
    pub const WATER: u32 = 5; // pictuniv.gob_2_10005
    pub const DESERT: u32 = 6; // pictuniv.gob_2_10006
    // Additional terrain types in atlas (not currently used in game):
    // Index 7: farmland (pictuniv.gob_2_10007)
    // Index 8: cotton (pictuniv.gob_2_10008)
    // Index 9: cattle (pictuniv.gob_2_10009)
    // Index 10: horses (pictuniv.gob_2_10012)
    // Index 11: orchard (pictuniv.gob_2_10015)
    // Index 12: sheep (pictuniv.gob_2_10028)
}

impl TerrainType {
    /// Get the texture index for this terrain type
    pub fn get_texture_index(&self) -> u32 {
        match self {
            TerrainType::Grass => TileIndex::GRASS,
            TerrainType::Water => TileIndex::WATER,
            TerrainType::Mountain => TileIndex::MOUNTAIN,
            TerrainType::Hills => TileIndex::HILLS,
            TerrainType::Desert => TileIndex::DESERT,
            TerrainType::Forest => TileIndex::FOREST,
            TerrainType::Swamp => TileIndex::SWAMP,
        }
    }
}
