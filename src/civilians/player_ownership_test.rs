use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use crate::civilians::commands::{SelectCivilian, SelectedCivilian};
use crate::civilians::systems::handle_civilian_selection;
use crate::civilians::types::{Civilian, CivilianId, CivilianKind};
use crate::economy::{NationId, PlayerNation};
use bevy_ecs_tilemap::prelude::TilePos;

#[test]
fn test_cannot_select_enemy_units() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<SelectedCivilian>();

    // Create player nation
    let player_nation_entity = world.spawn(NationId(1)).id();
    let player_instance =
        moonshine_kind::Instance::<NationId>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create enemy nation
    let enemy_nation_entity = world.spawn(NationId(2)).id();

    // Create an enemy civilian
    let enemy_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: enemy_nation_entity,
            owner_id: NationId(1),
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Manually write a SelectCivilian event to the message queue
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: enemy_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the selection system
    {
        let mut system_state: SystemState<(
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let (player_nation, events, selected, civilians) = system_state.get_mut(&mut world);
        handle_civilian_selection(player_nation, events, selected, civilians);
        system_state.apply(&mut world);
    }

    // Verify that the enemy unit was NOT selected
    let selected = world.resource::<SelectedCivilian>();
    assert!(
        selected.0 != Some(enemy_civilian_entity),
        "Enemy units should not be selectable by player"
    );
}

#[test]
fn test_can_select_own_units() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<SelectedCivilian>();

    // Create player nation
    let player_nation_entity = world.spawn(NationId(1)).id();
    let player_instance =
        moonshine_kind::Instance::<NationId>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create a player-owned civilian
    let player_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: player_nation_entity,
            owner_id: NationId(1),
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Manually write a SelectCivilian event to the message queue
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: player_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the selection system
    {
        let mut system_state: SystemState<(
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let (player_nation, events, selected, civilians) = system_state.get_mut(&mut world);
        handle_civilian_selection(player_nation, events, selected, civilians);
        system_state.apply(&mut world);
    }

    // Verify that the player unit WAS selected
    let selected = world.resource::<SelectedCivilian>();
    assert!(
        selected.0 == Some(player_civilian_entity),
        "Player should be able to select their own units"
    );
}

#[test]
fn test_selecting_player_unit_deselects_others() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<SelectedCivilian>();

    // Create player nation
    let player_nation_entity = world.spawn(NationId(1)).id();
    let player_instance =
        moonshine_kind::Instance::<NationId>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create first player-owned civilian
    let first_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: player_nation_entity,
            owner_id: NationId(1),
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
            owner_id: NationId(1),
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Select the first civilian first
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: first_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    {
        let mut system_state: SystemState<(
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let (player_nation, events, selected, civilians) = system_state.get_mut(&mut world);
        handle_civilian_selection(player_nation, events, selected, civilians);
        system_state.apply(&mut world);
    }

    // Select the second civilian
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: second_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the selection system
    {
        let mut system_state: SystemState<(
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
        )> = SystemState::new(&mut world);

        let (player_nation, events, selected, civilians) = system_state.get_mut(&mut world);
        handle_civilian_selection(player_nation, events, selected, civilians);
        system_state.apply(&mut world);
    }

    // Verify that only the second unit is selected
    let selected = world.resource::<SelectedCivilian>();
    assert!(
        selected.0 == Some(second_civilian_entity),
        "Second unit should be selected"
    );
    assert!(
        selected.0 != Some(first_civilian_entity),
        "First unit should be deselected when second unit is selected"
    );
}
