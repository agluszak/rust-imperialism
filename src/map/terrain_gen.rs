use crate::map::tiles::TerrainType;
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
    /// Returns a TerrainType based on multiple noise layers
    pub fn generate_terrain(
        &self,
        x: u32,
        y: u32,
        map_size_x: u32,
        map_size_y: u32,
    ) -> TerrainType {
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

        self.classify_terrain(elevation, moisture, temperature)
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

        // Hills: moderate-high elevation
        if elevation > 0.6 && elevation <= 0.7 {
            return TerrainType::Hills;
        }

        // For mid-elevation areas, use moisture and temperature with more nuanced classification
        if moisture > 0.6 && temperature > 0.4 {
            TerrainType::Forest // High moisture = forests
        } else if moisture < 0.35 && temperature > 0.6 {
            TerrainType::Desert // Low moisture + high temp = desert
        } else if moisture < 0.4 && elevation > 0.55 {
            TerrainType::Hills // Dry high areas = hills
        } else if moisture > 0.35 && moisture < 0.55 && temperature > 0.3 && temperature < 0.7 {
            // Moderate moisture and temperature = ideal farmland
            TerrainType::Farmland
        } else if moisture < 0.25 && temperature < 0.3 {
            TerrainType::Swamp // Low temp + low moisture in lowlands
        } else {
            TerrainType::Grass // Default grassland
        }
    }
}

impl Default for TerrainGenerator {
    fn default() -> Self {
        Self::new(42) // Default seed
    }
}
