use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

use super::commands::{DeselectAllCivilians, GiveCivilianOrder, RescindOrders, SelectCivilian};
use super::types::CivilianOrderKind;
use super::types::{Civilian, CivilianKind, PreviousPosition};
use crate::ui::button_style::*;

/// Marker for Engineer orders UI panel
#[derive(Component)]
pub struct EngineerOrdersPanel;

/// Marker for resource improver orders UI panel (Farmer, Rancher, etc.)
#[derive(Component)]
pub struct ImproverOrdersPanel;

/// Marker for rescind orders panel
#[derive(Component)]
pub struct RescindOrdersPanel;

/// Show/hide Engineer orders UI based on selection messages
/// Event-driven system that only runs when selection actually changes
pub fn update_engineer_orders_ui(
    mut commands: Commands,
    mut select_events: MessageReader<SelectCivilian>,
    mut deselect_all_events: MessageReader<DeselectAllCivilians>,
    civilians: Query<&Civilian>,
    existing_panel: Query<Entity, With<EngineerOrdersPanel>>,
) {
    // Handle deselect-all first (always hides panel)
    if !deselect_all_events.is_empty() {
        deselect_all_events.clear();
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Handle selection events
    let mut selected_engineer_entity = None;
    for event in select_events.read() {
        if let Ok(civilian) = civilians.get(event.entity)
            && civilian.kind == CivilianKind::Engineer
        {
            selected_engineer_entity = Some(event.entity);
        }
    }

    if let Some(civilian_entity) = selected_engineer_entity {
        // Engineer is selected, ensure panel exists
        if existing_panel.is_empty() {
            info!("Creating Engineer orders panel");
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(100.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                EngineerOrdersPanel,
                children![
                    (
                        Text::new("Engineer Orders"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ),
                    (
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                        observe(move |_: On<Activate>, mut order_writer: MessageWriter<GiveCivilianOrder>| {
                            info!("Build Depot button clicked for civilian {:?}", civilian_entity);
                            order_writer.write(GiveCivilianOrder {
                                entity: civilian_entity,
                                order: CivilianOrderKind::BuildDepot,
                            });
                        }),
                        children![(
                            Text::new("Build Depot"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        )],
                    ),
                    (
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                        observe(move |_: On<Activate>, mut order_writer: MessageWriter<GiveCivilianOrder>| {
                            info!("Build Port button clicked for civilian {:?}", civilian_entity);
                            order_writer.write(GiveCivilianOrder {
                                entity: civilian_entity,
                                order: CivilianOrderKind::BuildPort,
                            });
                        }),
                        children![(
                            Text::new("Build Port"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        )],
                    ),
                ],
            ));
        }
    } else if !select_events.is_empty() {
        // Non-engineer selected, remove panel if it exists
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Show/hide resource improver orders UI based on selection messages
/// Event-driven system that only runs when selection actually changes
pub fn update_improver_orders_ui(
    mut commands: Commands,
    mut select_events: MessageReader<SelectCivilian>,
    mut deselect_all_events: MessageReader<DeselectAllCivilians>,
    civilians: Query<&Civilian>,
    existing_panel: Query<Entity, With<ImproverOrdersPanel>>,
) {
    // Handle deselect-all first (always hides panel)
    if !deselect_all_events.is_empty() {
        deselect_all_events.clear();
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Handle selection events
    let mut selected_improver_data = None;
    for event in select_events.read() {
        if let Ok(civilian) = civilians.get(event.entity)
            && matches!(
                civilian.kind,
                CivilianKind::Farmer
                    | CivilianKind::Rancher
                    | CivilianKind::Forester
                    | CivilianKind::Miner
                    | CivilianKind::Driller
            )
        {
            selected_improver_data = Some((event.entity, civilian.kind));
        }
    }

    if let Some((civilian_entity, civilian_kind)) = selected_improver_data {
        // Resource improver is selected, ensure panel exists
        if existing_panel.is_empty() {
            let panel_title = format!("{:?} Orders", civilian_kind);
            info!("Creating {} orders panel", panel_title);
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(100.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.15, 0.1, 0.95)),
                ImproverOrdersPanel,
                children![
                    (
                        Text::new(panel_title),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ),
                    (
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                        observe(move |_: On<Activate>, mut order_writer: MessageWriter<GiveCivilianOrder>| {
                            info!("Improve Tile button clicked for {:?}", civilian_kind);
                            order_writer.write(GiveCivilianOrder {
                                entity: civilian_entity,
                                order: CivilianOrderKind::ImproveTile,
                            });
                        }),
                        children![(
                            Text::new("Improve Tile"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                        )],
                    ),
                ],
            ));
        }
    } else if !select_events.is_empty() {
        // Non-improver selected, remove panel if it exists
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Update UI for rescind orders panel based on selection messages
/// Event-driven system that only runs when selection actually changes
pub fn update_rescind_orders_ui(
    mut commands: Commands,
    mut select_events: MessageReader<SelectCivilian>,
    mut deselect_all_events: MessageReader<DeselectAllCivilians>,
    civilians_with_prev: Query<&PreviousPosition, With<Civilian>>,
    existing_panel: Query<Entity, With<RescindOrdersPanel>>,
) {
    // Handle deselect-all first (always hides panel)
    if !deselect_all_events.is_empty() {
        deselect_all_events.clear();
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Handle selection events - check if selected civilian has PreviousPosition
    let mut selected_data = None;
    for event in select_events.read() {
        if let Ok(prev_pos) = civilians_with_prev.get(event.entity) {
            selected_data = Some((event.entity, *prev_pos));
        }
    }

    if let Some((civilian_entity, prev_pos)) = selected_data {
        // Civilian is selected and has a previous position - show panel
        if existing_panel.is_empty() {
            // Create panel if it doesn't exist
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    bottom: Val::Px(200.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.12, 0.1, 0.95)),
                BorderColor::all(Color::srgba(0.6, 0.5, 0.4, 0.9)),
                RescindOrdersPanel,
                children![
                    (
                        Text::new(format!(
                            "Undo Action\nWas at: ({}, {})",
                            prev_pos.0.x, prev_pos.0.y
                        )),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.9, 0.7)),
                    ),
                    (
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_DANGER),
                        crate::ui::button_style::DangerButton,
                        observe(move |_: On<Activate>, mut rescind_writer: MessageWriter<RescindOrders>| {
                            info!("Rescind Orders button clicked for civilian {:?}", civilian_entity);
                            rescind_writer.write(RescindOrders {
                                entity: civilian_entity,
                            });
                        }),
                        children![(
                            Text::new("Rescind Orders"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.9, 0.9)),
                        )],
                    ),
                    (
                        Text::new("(Refund if same turn)"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.9, 0.7)),
                    ),
                ],
            ));
        }
    } else if !select_events.is_empty() {
        // Selected civilian without previous position, remove panel if it exists
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
    }
}
