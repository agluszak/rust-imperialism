use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_kind::Instance;

/// Unique identifier for a nation (stable across saves)
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Hash, Reflect)]
#[reflect(Component)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nation_id_equality() {
        let id1 = NationId(1);
        let id2 = NationId(1);
        let id3 = NationId(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn nation_id_hash() {
        use std::collections::HashMap;

        let id1 = NationId(1);
        let id2 = NationId(1);
        let id3 = NationId(2);

        let mut map = HashMap::new();
        map.insert(id1, "Nation 1");
        map.insert(id3, "Nation 2");

        assert_eq!(map.get(&id2), Some(&"Nation 1"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn player_nation_from_entity_with_valid_nation() {
        let mut world = World::new();

        // Create a nation entity with NationId
        let nation_entity = world
            .spawn((NationId(42), Name("Test Nation".to_string())))
            .id();

        // PlayerNation::from_entity should succeed
        let player_nation = PlayerNation::from_entity(&world, nation_entity);
        assert!(player_nation.is_some());

        let player_nation = player_nation.unwrap();
        assert_eq!(player_nation.entity(), nation_entity);

        // Verify we can access the NationId through the instance
        let nation_id = world.entity(player_nation.entity()).get::<NationId>();
        assert_eq!(nation_id.unwrap().0, 42);
    }

    #[test]
    fn player_nation_from_entity_without_nation_id() {
        let mut world = World::new();

        // Create an entity WITHOUT NationId
        let non_nation_entity = world.spawn(Name("Not a Nation".to_string())).id();

        // PlayerNation::from_entity should return None
        let player_nation = PlayerNation::from_entity(&world, non_nation_entity);
        assert!(player_nation.is_none());
    }

    #[test]
    fn player_nation_from_entity_with_invalid_entity() {
        let world = World::new();

        // Try to create PlayerNation from an entity that doesn't exist
        let invalid_entity = Entity::from_bits(999999);
        let player_nation = PlayerNation::from_entity(&world, invalid_entity);

        assert!(player_nation.is_none());
    }

    #[test]
    fn player_nation_new_and_accessors() {
        let mut world = World::new();

        let nation_entity = world.spawn(NationId(7)).id();
        let instance = Instance::<NationId>::from_entity(world.entity(nation_entity)).unwrap();

        let player_nation = PlayerNation::new(instance);

        // Test accessors
        assert_eq!(player_nation.entity(), nation_entity);
        assert_eq!(player_nation.instance().entity(), nation_entity);
    }

    #[test]
    fn player_nation_roundtrip() {
        let mut world = World::new();

        // Create a nation
        let nation_entity = world
            .spawn((NationId(123), Name("Player Nation".to_string())))
            .id();

        // Create PlayerNation from entity
        let player_nation = PlayerNation::from_entity(&world, nation_entity)
            .expect("Should create PlayerNation from valid nation entity");

        // Extract entity and verify it matches
        assert_eq!(player_nation.entity(), nation_entity);

        // Verify we can still access the NationId component
        let stored_id = world
            .entity(player_nation.entity())
            .get::<NationId>()
            .unwrap();
        assert_eq!(stored_id.0, 123);
    }

    #[test]
    fn multiple_nations_with_different_ids() {
        let mut world = World::new();

        let nation1 = world.spawn(NationId(1)).id();
        let nation2 = world.spawn(NationId(2)).id();
        let nation3 = world.spawn(NationId(3)).id();

        // Create PlayerNation references to each
        let player1 = PlayerNation::from_entity(&world, nation1).unwrap();
        let player2 = PlayerNation::from_entity(&world, nation2).unwrap();
        let player3 = PlayerNation::from_entity(&world, nation3).unwrap();

        // Verify each points to the correct entity
        assert_eq!(player1.entity(), nation1);
        assert_eq!(player2.entity(), nation2);
        assert_eq!(player3.entity(), nation3);

        // Verify the IDs are correct
        assert_eq!(
            world.entity(player1.entity()).get::<NationId>().unwrap().0,
            1
        );
        assert_eq!(
            world.entity(player2.entity()).get::<NationId>().unwrap().0,
            2
        );
        assert_eq!(
            world.entity(player3.entity()).get::<NationId>().unwrap().0,
            3
        );
    }

    #[test]
    fn player_nation_survives_entity_despawn() {
        let mut world = World::new();

        let nation_entity = world.spawn(NationId(50)).id();
        let player_nation = PlayerNation::from_entity(&world, nation_entity).unwrap();

        // Despawn the entity
        world.despawn(nation_entity);

        // PlayerNation still holds the entity reference (now invalid)
        // This tests that PlayerNation is just a handle - it doesn't prevent despawning
        assert_eq!(player_nation.entity(), nation_entity);

        // Trying to access the entity should fail
        assert!(world.get_entity(player_nation.entity()).is_err());
    }

    #[test]
    fn name_component() {
        let name1 = Name("Test".to_string());
        let name2 = Name("Test".to_string());

        // Name uses String, which implements PartialEq
        assert_eq!(name1.0, name2.0);
    }

    #[test]
    fn capital_component() {
        let capital1 = Capital(TilePos { x: 10, y: 20 });
        let capital2 = Capital(TilePos { x: 10, y: 20 });
        let capital3 = Capital(TilePos { x: 15, y: 25 });

        // Test Copy trait
        let capital_copy = capital1;
        assert_eq!(capital_copy.0.x, 10);
        assert_eq!(capital_copy.0.y, 20);

        // Test that TilePos comparison works
        assert_eq!(capital1.0, capital2.0);
        assert_ne!(capital1.0, capital3.0);
    }
}
