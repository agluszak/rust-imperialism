use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use crate::civilians::commands::SelectCivilian;
use crate::civilians::systems::handle_civilian_selection;
use crate::civilians::types::{Civilian, CivilianKind, Selected, SelectedCivilian};
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
            owner_id: NationId(2),
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
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
            Query<Entity, With<Selected>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, events, selected, civilians, marked) =
            system_state.get_mut(&mut world);
        handle_civilian_selection(commands, player_nation, events, selected, civilians, marked);
        system_state.apply(&mut world);
    }

    // Verify that the enemy unit was NOT selected
    let selected = world.resource::<SelectedCivilian>();
    assert!(
        selected.0.is_none(),
        "Enemy units should not be selectable by player"
    );
    
    // Verify the Selected marker was not added
    assert!(
        world.get::<Selected>(enemy_civilian_entity).is_none(),
        "Selected marker should not be added to enemy units"
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
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
            Query<Entity, With<Selected>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, events, selected, civilians, marked) =
            system_state.get_mut(&mut world);
        handle_civilian_selection(commands, player_nation, events, selected, civilians, marked);
        system_state.apply(&mut world);
    }

    // Verify that the player unit WAS selected
    let selected = world.resource::<SelectedCivilian>();
    assert_eq!(
        selected.0,
        Some(player_civilian_entity),
        "Player should be able to select their own units"
    );
    
    // Verify the Selected marker was added
    assert!(
        world.get::<Selected>(player_civilian_entity).is_some(),
        "Selected marker should be added to player units"
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

    // Create first player-owned civilian (already selected)
    let first_civilian_entity = world
        .spawn((
            Civilian {
                kind: CivilianKind::Engineer,
                position: TilePos { x: 0, y: 0 },
                owner: player_nation_entity,
                owner_id: NationId(1),
                has_moved: false,
            },
            Selected, // Mark as selected
        ))
        .id();
    
    // Set the resource to reflect first civilian is selected
    world.resource_mut::<SelectedCivilian>().0 = Some(first_civilian_entity);

    // Create second player-owned civilian
    let second_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Prospector,
            position: TilePos { x: 1, y: 1 },
            owner: player_nation_entity,
            owner_id: NationId(1),
            has_moved: false,
        })
        .id();

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
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            ResMut<SelectedCivilian>,
            Query<&Civilian>,
            Query<Entity, With<Selected>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, events, selected, civilians, marked) =
            system_state.get_mut(&mut world);
        handle_civilian_selection(commands, player_nation, events, selected, civilians, marked);
        system_state.apply(&mut world);
    }

    // Verify that the second unit is selected and first unit is deselected
    let selected = world.resource::<SelectedCivilian>();
    assert_eq!(
        selected.0,
        Some(second_civilian_entity),
        "Second unit should be selected"
    );
    
    // Verify markers
    assert!(
        world.get::<Selected>(first_civilian_entity).is_none(),
        "First unit should have Selected marker removed"
    );
    assert!(
        world.get::<Selected>(second_civilian_entity).is_some(),
        "Second unit should have Selected marker"
    );
}
