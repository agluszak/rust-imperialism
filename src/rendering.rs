use bevy::prelude::*;

/// Points from any map sprite entity to the game entity it visualizes
/// This is a universal relationship that works for all map entity types:
/// civilians, cities, depots, ports, regiments, etc.
#[derive(Component)]
#[relationship(relationship_target = MapVisual)]
pub struct MapVisualFor(pub Entity);

/// Auto-maintained component on game entities that points to their map sprite
/// Do not modify directly - automatically updated via relationship hooks
#[derive(Component)]
#[relationship_target(relationship = MapVisualFor)]
pub struct MapVisual(Entity);

impl MapVisual {
    /// Get the entity of the sprite
    pub fn entity(&self) -> Entity {
        self.0
    }
}
