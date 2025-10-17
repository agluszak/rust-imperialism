use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_kind::Instance;

/// Unique identifier for a nation (stable across saves)
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NationId(pub u16);

/// Display name for a nation
#[derive(Component, Clone, Debug)]
pub struct Name(pub String);

/// Type-safe handle to a nation entity.
pub type NationInstance = Instance<NationId>;

/// Capital tile position for a nation (used for rail network connectivity)
#[derive(Component, Clone, Copy, Debug)]
pub struct Capital(pub TilePos);

/// Resource pointing to the player's active nation entity
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayerNation(pub NationInstance);

impl PlayerNation {
    /// Creates a new resource from the given nation instance.
    pub fn new(instance: NationInstance) -> Self {
        Self(instance)
    }

    /// Attempts to create a player nation reference from the given entity.
    ///
    /// Returns [`None`] if the entity does not contain a [`NationId`] component.
    pub fn from_entity(world: &World, entity: Entity) -> Option<Self> {
        world
            .get_entity(entity)
            .ok()
            .and_then(Instance::<NationId>::from_entity)
            .map(Self)
    }

    /// Returns the instance of the player's nation.
    pub fn instance(&self) -> NationInstance {
        self.0
    }

    /// Returns the underlying entity for the player's nation.
    pub fn entity(&self) -> Entity {
        self.0.entity()
    }
}

/// Nation display color (for borders and UI)
#[derive(Component, Clone, Copy, Debug)]
pub struct NationColor(pub Color);
