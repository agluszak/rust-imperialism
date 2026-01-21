use bevy::prelude::*;

use crate::civilians::commands::{SelectCivilian, SelectedCivilian};
use crate::civilians::systems::handle_civilian_selection;
use crate::civilians::types::{Civilian, CivilianId, CivilianKind};
use crate::economy::{Nation, PlayerNation};
use bevy_ecs_tilemap::prelude::TilePos;

#[test]
fn test_cannot_select_enemy_units() {
    let mut world = World::new();
    world.add_observer(handle_civilian_selection);

    // Create player nation
    let player_nation_entity = world.spawn(Nation).id();
    let player_instance =
        moonshine_kind::Instance::<Nation>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create enemy nation
    let enemy_nation_entity = world.spawn(Nation).id();

    // Create an enemy civilian
    let enemy_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: enemy_nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Trigger SelectCivilian event
    world.trigger(SelectCivilian {
        entity: enemy_civilian_entity,
    });
    world.flush();

    // Verify that the enemy unit was NOT selected
    assert!(
        world.get_resource::<SelectedCivilian>().is_none(),
        "Enemy units should not be selectable by player"
    );
}

#[test]
fn test_can_select_own_units() {
    let mut world = World::new();
    world.add_observer(handle_civilian_selection);

    // Create player nation
    let player_nation_entity = world.spawn(Nation).id();
    let player_instance =
        moonshine_kind::Instance::<Nation>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create a player-owned civilian
    let player_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: player_nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Trigger SelectCivilian event
    world.trigger(SelectCivilian {
        entity: player_civilian_entity,
    });
    world.flush();

    // Verify that the player unit WAS selected
    let selected = world
        .get_resource::<SelectedCivilian>()
        .expect("Player unit should be selected");
    assert!(
        selected.0 == player_civilian_entity,
        "Player should be able to select their own units"
    );
}

#[test]
fn test_selecting_player_unit_deselects_others() {
    let mut world = World::new();
    world.add_observer(handle_civilian_selection);

    // Create player nation
    let player_nation_entity = world.spawn(Nation).id();
    let player_instance =
        moonshine_kind::Instance::<Nation>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create first player-owned civilian
    let first_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: player_nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Create second player-owned civilian
    let second_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Prospector,
            position: TilePos { x: 1, y: 1 },
            owner: player_nation_entity,
            civilian_id: CivilianId(1),
            has_moved: false,
        })
        .id();

    // Select the first civilian first
    world.trigger(SelectCivilian {
        entity: first_civilian_entity,
    });
    world.flush();

    // Select the second civilian
    world.trigger(SelectCivilian {
        entity: second_civilian_entity,
    });
    world.flush();

    // Verify that only the second unit is selected
    let selected = world
        .get_resource::<SelectedCivilian>()
        .expect("Second unit should be selected");
    assert!(
        selected.0 == second_civilian_entity,
        "Second unit should be selected"
    );
    assert!(
        selected.0 != first_civilian_entity,
        "First unit should be deselected when second unit is selected"
    );
}
