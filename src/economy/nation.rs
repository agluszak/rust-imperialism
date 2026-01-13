use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use moonshine_kind::Instance;
use moonshine_save::prelude::Save;

/// Marker component for nation entities.
/// Used with moonshine_kind::Instance for type-safe nation references.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(Save, Name)]
pub struct Nation;

/// Relationship from any game entity to the nation that owns it.
/// Used for civilians, cities, provinces, depots, ports, ships, etc.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
#[require(Save)]
#[relationship(relationship_target = NationMember)]
pub struct OwnedBy(pub Entity);

/// Auto-maintained target marker for entities owned by a nation.
/// Automatically added/removed when OwnedBy relationship is changed.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
#[relationship_target(relationship = OwnedBy)]
pub struct NationMember(Entity);

impl NationMember {
    pub fn nation(&self) -> Entity {
        self.0
    }
}

/// Type-safe handle to a nation entity.
/// Can be used directly in queries: `Query<(NationInstance, &Name)>`
pub type NationInstance = Instance<Nation>;

/// Capital tile position for a nation (used for rail network connectivity)
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
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
    /// Returns [`None`] if the entity does not contain a [`Nation`] component.
    pub fn from_entity(world: &World, entity: Entity) -> Option<Self> {
        world
            .get_entity(entity)
            .ok()
            .and_then(Instance::<Nation>::from_entity)
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
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct NationColor(pub Color);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_nation_from_entity_with_valid_nation() {
        let mut world = World::new();

        // Create a nation entity with Nation marker
        let nation_entity = world.spawn((Nation, Name::new("Test Nation"))).id();

        // PlayerNation::from_entity should succeed
        let player_nation = PlayerNation::from_entity(&world, nation_entity);
        assert!(player_nation.is_some());

        let player_nation = player_nation.unwrap();
        assert_eq!(player_nation.entity(), nation_entity);

        // Verify we can access the Nation marker through the instance
        let nation = world.entity(player_nation.entity()).get::<Nation>();
        assert!(nation.is_some());
    }

    #[test]
    fn player_nation_from_entity_without_nation_marker() {
        let mut world = World::new();

        // Create an entity WITHOUT Nation marker
        let non_nation_entity = world.spawn(Name::new("Not a Nation")).id();

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

        let nation_entity = world.spawn(Nation).id();
        let instance = Instance::<Nation>::from_entity(world.entity(nation_entity)).unwrap();

        let player_nation = PlayerNation::new(instance);

        // Test accessors
        assert_eq!(player_nation.entity(), nation_entity);
        assert_eq!(player_nation.instance().entity(), nation_entity);
    }

    #[test]
    fn player_nation_roundtrip() {
        let mut world = World::new();

        // Create a nation
        let nation_entity = world.spawn((Nation, Name::new("Player Nation"))).id();

        // Create PlayerNation from entity
        let player_nation = PlayerNation::from_entity(&world, nation_entity)
            .expect("Should create PlayerNation from valid nation entity");

        // Extract entity and verify it matches
        assert_eq!(player_nation.entity(), nation_entity);

        // Verify we can still access the Nation component
        let nation = world.entity(player_nation.entity()).get::<Nation>();
        assert!(nation.is_some());
    }

    #[test]
    fn multiple_nations() {
        let mut world = World::new();

        let nation1 = world.spawn(Nation).id();
        let nation2 = world.spawn(Nation).id();
        let nation3 = world.spawn(Nation).id();

        // Create PlayerNation references to each
        let player1 = PlayerNation::from_entity(&world, nation1).unwrap();
        let player2 = PlayerNation::from_entity(&world, nation2).unwrap();
        let player3 = PlayerNation::from_entity(&world, nation3).unwrap();

        // Verify each points to the correct entity
        assert_eq!(player1.entity(), nation1);
        assert_eq!(player2.entity(), nation2);
        assert_eq!(player3.entity(), nation3);
    }

    #[test]
    fn player_nation_survives_entity_despawn() {
        let mut world = World::new();

        let nation_entity = world.spawn(Nation).id();
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
        let name1 = Name::new("Test");
        let name2 = Name::new("Test");

        // Name implements PartialEq
        assert_eq!(name1, name2);
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

    #[test]
    fn nation_instance_can_be_used_as_hashmap_key() {
        use std::collections::HashMap;

        let mut world = World::new();

        let nation1 = world.spawn(Nation).id();
        let nation2 = world.spawn(Nation).id();

        let instance1 = Instance::<Nation>::from_entity(world.entity(nation1)).unwrap();
        let instance2 = Instance::<Nation>::from_entity(world.entity(nation2)).unwrap();

        let mut map: HashMap<NationInstance, &str> = HashMap::new();
        map.insert(instance1, "Nation 1");
        map.insert(instance2, "Nation 2");

        assert_eq!(map.get(&instance1), Some(&"Nation 1"));
        assert_eq!(map.get(&instance2), Some(&"Nation 2"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_nation_requires_name() {
        let mut world = World::new();
        let entity = world.spawn(Nation).id();

        // Name should be automatically added by Bevy's require feature
        let name = world
            .get::<Name>(entity)
            .expect("Name component should be required by Nation");
        assert_eq!(name.as_str(), "");
    }
}
