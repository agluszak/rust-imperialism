use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use crate::civilians::commands::{DeselectCivilian, SelectCivilian};
use crate::civilians::types::{Civilian, CivilianId, CivilianKind};
use crate::civilians::ui_components::{update_civilian_orders_ui, CivilianOrdersPanel};
use crate::economy::{Nation, PlayerNation};
use bevy_ecs_tilemap::prelude::TilePos;

/// Test that civilian orders UI is NOT shown for enemy units
#[test]
fn test_ui_not_shown_for_enemy_units() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create player nation
    let player_nation_entity = world.spawn(Nation).id();
    let player_instance =
        moonshine_kind::Instance::<Nation>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create enemy nation
    let enemy_nation_entity = world.spawn(Nation).id();

    // Create an enemy engineer (which normally shows orders panel)
    let enemy_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: enemy_nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Send SelectCivilian event for enemy unit
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: enemy_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the UI update system
    {
        let mut system_state: SystemState<(
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            MessageReader<DeselectCivilian>,
            Query<&Civilian>,
            Query<Entity, With<CivilianOrdersPanel>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, select_events, deselect_events, civilians, existing_panel) =
            system_state.get_mut(&mut world);
        update_civilian_orders_ui(
            commands,
            player_nation,
            select_events,
            deselect_events,
            civilians,
            existing_panel,
        );
        system_state.apply(&mut world);
    }

    // Verify that NO UI panel was created
    let panels: Vec<Entity> = world
        .query_filtered::<Entity, With<CivilianOrdersPanel>>()
        .iter(&world)
        .collect();

    assert!(
        panels.is_empty(),
        "UI panel should NOT be shown for enemy units"
    );
}

/// Test that civilian orders UI IS shown for player-owned units
#[test]
fn test_ui_shown_for_player_units() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // Create player nation
    let player_nation_entity = world.spawn(Nation).id();
    let player_instance =
        moonshine_kind::Instance::<Nation>::from_entity(world.entity(player_nation_entity))
            .unwrap();
    world.insert_resource(PlayerNation::new(player_instance));

    // Create a player-owned engineer (which should show orders panel)
    let player_civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: player_nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Send SelectCivilian event for player unit
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: player_civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the UI update system
    {
        let mut system_state: SystemState<(
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            MessageReader<DeselectCivilian>,
            Query<&Civilian>,
            Query<Entity, With<CivilianOrdersPanel>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, select_events, deselect_events, civilians, existing_panel) =
            system_state.get_mut(&mut world);
        update_civilian_orders_ui(
            commands,
            player_nation,
            select_events,
            deselect_events,
            civilians,
            existing_panel,
        );
        system_state.apply(&mut world);
    }

    // Verify that a UI panel WAS created
    let panels: Vec<Entity> = world
        .query_filtered::<Entity, With<CivilianOrdersPanel>>()
        .iter(&world)
        .collect();

    assert_eq!(
        panels.len(),
        1,
        "UI panel should be shown for player-owned units"
    );
}

/// Test that UI is not shown when there is no player nation
#[test]
fn test_ui_not_shown_without_player_nation() {
    let mut world = World::new();
    world.init_resource::<Messages<SelectCivilian>>();
    world.init_resource::<Messages<DeselectCivilian>>();

    // DO NOT set PlayerNation resource

    // Create a nation
    let nation_entity = world.spawn(Nation).id();

    // Create a civilian
    let civilian_entity = world
        .spawn(Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 0, y: 0 },
            owner: nation_entity,
            civilian_id: CivilianId(0),
            has_moved: false,
        })
        .id();

    // Send SelectCivilian event
    {
        let mut system_state: SystemState<MessageWriter<SelectCivilian>> =
            SystemState::new(&mut world);
        let mut writer = system_state.get_mut(&mut world);
        writer.write(SelectCivilian {
            entity: civilian_entity,
        });
        system_state.apply(&mut world);
    }

    // Run the UI update system
    {
        let mut system_state: SystemState<(
            Commands,
            Option<Res<PlayerNation>>,
            MessageReader<SelectCivilian>,
            MessageReader<DeselectCivilian>,
            Query<&Civilian>,
            Query<Entity, With<CivilianOrdersPanel>>,
        )> = SystemState::new(&mut world);

        let (commands, player_nation, select_events, deselect_events, civilians, existing_panel) =
            system_state.get_mut(&mut world);
        update_civilian_orders_ui(
            commands,
            player_nation,
            select_events,
            deselect_events,
            civilians,
            existing_panel,
        );
        system_state.apply(&mut world);
    }

    // Verify that NO UI panel was created
    let panels: Vec<Entity> = world
        .query_filtered::<Entity, With<CivilianOrdersPanel>>()
        .iter(&world)
        .collect();

    assert!(
        panels.is_empty(),
        "UI panel should NOT be shown when there is no player nation"
    );
}
