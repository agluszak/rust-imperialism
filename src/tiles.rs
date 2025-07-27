use bevy::prelude::*;

/// Component to identify the type and properties of a tile
#[derive(Component, Debug, Clone, PartialEq)]
pub struct TileType {
    pub category: TileCategory,
    pub properties: TileProperties,
}

/// Main categories of tiles based on Kenney's colored_packed.png
#[derive(Debug, Clone, PartialEq)]
pub enum TileCategory {
    // Terrain types - most important for imperialism game
    Terrain(TerrainType),
    // Military units and structures
    Military(MilitaryType),
    // Resource tiles
    Resource(ResourceType),
    // Buildings and infrastructure
    Building(BuildingType),
    // UI and interface elements
    UI(UIType),
}

/// Different terrain types for map generation
#[derive(Debug, Clone, PartialEq)]
pub enum TerrainType {
    Grass,    // Plains - good for agriculture
    Water,    // Ocean/rivers - naval movement
    Mountain, // High ground - defensive bonus
    Desert,   // Harsh terrain - movement penalty
    Forest,   // Dense vegetation - hiding bonus
    Snow,     // Cold climate - harsh conditions
    Hills,    // Rolling terrain - minor defensive bonus
    Swamp,    // Wetlands - movement penalty
}

/// Military units and fortifications
#[derive(Debug, Clone, PartialEq)]
pub enum MilitaryType {
    Infantry,  // Basic ground troops
    Cavalry,   // Fast moving units
    Artillery, // Long range siege weapons
    Navy,      // Naval vessels
    Fortress,  // Heavy fortification
    Barracks,  // Unit production
    Arsenal,   // Weapon storage
    Wall,      // Defensive structure
}

/// Resources for empire building
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Gold,  // Currency
    Food,  // Population support
    Iron,  // Military production
    Wood,  // Construction material
    Stone, // Fortification material
    Gems,  // Luxury goods
    Trade, // Commerce routes
}

/// Buildings and infrastructure
#[derive(Debug, Clone, PartialEq)]
pub enum BuildingType {
    City,    // Population center
    Capital, // Empire capital
    Farm,    // Food production
    Mine,    // Resource extraction
    Market,  // Trade center
    Road,    // Transportation
    Bridge,  // River crossing
    Port,    // Naval base
}

/// UI and interface elements
#[derive(Debug, Clone, PartialEq)]
pub enum UIType {
    Button,
    Arrow,
    Number,
    Letter,
    Icon,
    Border,
    Panel,
}

/// Properties that affect gameplay
#[derive(Debug, Clone, PartialEq)]
pub struct TileProperties {
    pub movement_cost: f32,       // Cost to move through this tile
    pub defense_bonus: f32,       // Defensive advantage
    pub resource_yield: f32,      // Resource production
    pub population_capacity: u32, // How many people can live here
    pub is_passable: bool,        // Can units move through
    pub is_buildable: bool,       // Can structures be built
}

impl Default for TileProperties {
    fn default() -> Self {
        Self {
            movement_cost: 1.0,
            defense_bonus: 0.0,
            resource_yield: 0.0,
            population_capacity: 0,
            is_passable: true,
            is_buildable: true,
        }
    }
}

/// Predefined tile types with their indices in the colored_packed.png
pub struct TileIndex;

impl TileIndex {
    // Terrain tile indices (approximate based on the tileset layout)
    pub const GRASS: u32 = 0; // Green grass tile
    pub const WATER: u32 = 100; // Blue water tile
    pub const MOUNTAIN: u32 = 200; // Brown mountain tile
    pub const DESERT: u32 = 300; // Tan desert tile
    pub const FOREST: u32 = 400; // Dark green forest
    pub const SNOW: u32 = 500; // White snow tile

    // Military structures
    pub const FORTRESS: u32 = 600;
    pub const BARRACKS: u32 = 650;
    pub const WALL: u32 = 700;

    // Resources
    pub const GOLD: u32 = 800;
    pub const IRON: u32 = 850;
    pub const WOOD: u32 = 900;

    // Buildings
    pub const CITY: u32 = 950;
    pub const FARM: u32 = 1000;
    pub const MARKET: u32 = 1050;
}

impl TileType {
    /// Create a new terrain tile
    pub fn terrain(terrain_type: TerrainType) -> Self {
        let properties = match terrain_type {
            TerrainType::Grass => TileProperties {
                movement_cost: 1.0,
                defense_bonus: 0.0,
                resource_yield: 2.0, // Good for farming
                population_capacity: 10,
                is_passable: true,
                is_buildable: true,
            },
            TerrainType::Water => TileProperties {
                movement_cost: 2.0, // Naval movement
                defense_bonus: -1.0,
                resource_yield: 1.0, // Fishing
                population_capacity: 0,
                is_passable: false, // Need ships
                is_buildable: false,
            },
            TerrainType::Mountain => TileProperties {
                movement_cost: 3.0,
                defense_bonus: 2.0,  // High ground advantage
                resource_yield: 1.0, // Mining
                population_capacity: 2,
                is_passable: true,
                is_buildable: false,
            },
            TerrainType::Desert => TileProperties {
                movement_cost: 2.0,
                defense_bonus: 0.0,
                resource_yield: 0.5, // Harsh conditions
                population_capacity: 1,
                is_passable: true,
                is_buildable: true,
            },
            TerrainType::Forest => TileProperties {
                movement_cost: 2.0,
                defense_bonus: 1.0,  // Cover advantage
                resource_yield: 1.5, // Wood production
                population_capacity: 5,
                is_passable: true,
                is_buildable: true,
            },
            TerrainType::Snow => TileProperties {
                movement_cost: 2.5,
                defense_bonus: 0.5,
                resource_yield: 0.5, // Cold climate
                population_capacity: 1,
                is_passable: true,
                is_buildable: true,
            },
            TerrainType::Hills => TileProperties {
                movement_cost: 1.5,
                defense_bonus: 1.0,
                resource_yield: 1.0,
                population_capacity: 5,
                is_passable: true,
                is_buildable: true,
            },
            TerrainType::Swamp => TileProperties {
                movement_cost: 3.0,
                defense_bonus: -0.5,
                resource_yield: 0.5,
                population_capacity: 1,
                is_passable: true,
                is_buildable: false,
            },
        };

        Self {
            category: TileCategory::Terrain(terrain_type),
            properties,
        }
    }

    /// Create a military structure tile
    pub fn military(military_type: MilitaryType) -> Self {
        let properties = match military_type {
            MilitaryType::Fortress => TileProperties {
                movement_cost: 1.0,
                defense_bonus: 3.0,
                resource_yield: 0.0,
                population_capacity: 20,
                is_passable: true,
                is_buildable: false,
            },
            MilitaryType::Barracks => TileProperties {
                movement_cost: 1.0,
                defense_bonus: 1.0,
                resource_yield: 0.0,
                population_capacity: 10,
                is_passable: true,
                is_buildable: false,
            },
            _ => TileProperties::default(),
        };

        Self {
            category: TileCategory::Military(military_type),
            properties,
        }
    }

    /// Get the recommended texture index for this tile type
    pub fn get_texture_index(&self) -> u32 {
        match &self.category {
            TileCategory::Terrain(terrain) => match terrain {
                TerrainType::Grass => TileIndex::GRASS,
                TerrainType::Water => TileIndex::WATER,
                TerrainType::Mountain => TileIndex::MOUNTAIN,
                TerrainType::Desert => TileIndex::DESERT,
                TerrainType::Forest => TileIndex::FOREST,
                TerrainType::Snow => TileIndex::SNOW,
                _ => TileIndex::GRASS, // Default fallback
            },
            TileCategory::Military(military) => match military {
                MilitaryType::Fortress => TileIndex::FORTRESS,
                MilitaryType::Barracks => TileIndex::BARRACKS,
                MilitaryType::Wall => TileIndex::WALL,
                _ => TileIndex::FORTRESS,
            },
            TileCategory::Resource(resource) => match resource {
                ResourceType::Gold => TileIndex::GOLD,
                ResourceType::Iron => TileIndex::IRON,
                ResourceType::Wood => TileIndex::WOOD,
                _ => TileIndex::GOLD,
            },
            TileCategory::Building(building) => match building {
                BuildingType::City => TileIndex::CITY,
                BuildingType::Farm => TileIndex::FARM,
                BuildingType::Market => TileIndex::MARKET,
                _ => TileIndex::CITY,
            },
            TileCategory::UI(_) => 0, // UI elements use dynamic indices
        }
    }
}
