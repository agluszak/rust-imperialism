use crate::tiles::{TerrainType, TileType};
use noise::{NoiseFn, Perlin};

pub struct TerrainGenerator {
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            // Use different seeds for each noise layer for variety
            elevation_noise: Perlin::new(seed),
            moisture_noise: Perlin::new(seed.wrapping_add(1000)),
            temperature_noise: Perlin::new(seed.wrapping_add(2000)),
        }
    }

    /// Generate terrain for a given tile position
    /// Returns a TileType based on multiple noise layers
    pub fn generate_terrain(&self, x: u32, y: u32, map_size_x: u32, map_size_y: u32) -> TileType {
        // Normalize coordinates to [0, 1] range
        let norm_x = x as f64 / map_size_x as f64;
        let norm_y = y as f64 / map_size_y as f64;

        // Different scales for varied terrain features
        let elevation_scale = 4.0; // Larger features for elevation
        let moisture_scale = 6.0; // Medium features for moisture
        let temperature_scale = 8.0; // Smaller features for temperature variation

        // Generate noise values (-1 to 1, then normalize to 0 to 1)
        let elevation = (self
            .elevation_noise
            .get([norm_x * elevation_scale, norm_y * elevation_scale])
            + 1.0)
            / 2.0;
        let moisture = (self
            .moisture_noise
            .get([norm_x * moisture_scale, norm_y * moisture_scale])
            + 1.0)
            / 2.0;
        let temperature = (self
            .temperature_noise
            .get([norm_x * temperature_scale, norm_y * temperature_scale])
            + 1.0)
            / 2.0;

        // Combine noise layers to determine terrain type
        let terrain_type = self.classify_terrain(elevation, moisture, temperature);

        TileType::terrain(terrain_type)
    }

    /// Classify terrain based on elevation, moisture, and temperature
    fn classify_terrain(&self, elevation: f64, moisture: f64, temperature: f64) -> TerrainType {
        // Water: low elevation (more common)
        if elevation < 0.3 {
            return TerrainType::Water;
        }

        // Mountains: high elevation (less common but still present)
        if elevation > 0.7 {
            return TerrainType::Mountain;
        }

        // For mid-elevation areas, use moisture and temperature with more nuanced classification
        if moisture > 0.6 && temperature > 0.4 {
            TerrainType::Forest // High moisture = forests
        } else if moisture < 0.35 && temperature > 0.6 {
            TerrainType::Desert // Low moisture + high temp = desert
        } else if moisture < 0.4 && elevation > 0.55 {
            TerrainType::Mountain // Dry high areas = rocky mountains
        } else {
            TerrainType::Grass // Default grassland
        }
    }

    /// Get a preview of what terrain would be generated (useful for debugging)
    pub fn get_terrain_preview(&self, map_size_x: u32, map_size_y: u32) -> Vec<Vec<TerrainType>> {
        let mut preview = Vec::new();

        for y in 0..map_size_y {
            let mut row = Vec::new();
            for x in 0..map_size_x {
                let tile = self.generate_terrain(x, y, map_size_x, map_size_y);
                if let crate::tiles::TileCategory::Terrain(terrain_type) = tile.category {
                    row.push(terrain_type);
                }
            }
            preview.push(row);
        }

        preview
    }
}

impl Default for TerrainGenerator {
    fn default() -> Self {
        Self::new(42) // Default seed
    }
}
