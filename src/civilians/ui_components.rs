use bevy::prelude::*;

use super::commands::{DeselectAllCivilians, GiveCivilianOrder, RescindOrders, SelectCivilian};
use super::types::CivilianOrderKind;
use super::types::{Civilian, CivilianKind, PreviousPosition};
use crate::ui::button_style::*;

/// Marker for Engineer orders UI panel
#[derive(Component)]
pub struct EngineerOrdersPanel;

/// Marker for Build Depot button
#[derive(Component)]
pub struct BuildDepotButton;

/// Marker for Build Port button
#[derive(Component)]
pub struct BuildPortButton;

/// Marker for resource improver orders UI panel (Farmer, Rancher, etc.)
#[derive(Component)]
pub struct ImproverOrdersPanel;

/// Marker for Improve Tile button
#[derive(Component)]
pub struct ImproveTileButton;

/// Marker for Rescind Orders button
#[derive(Component)]
pub struct RescindOrdersButton;

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
    let mut selected_engineer = None;
    for event in select_events.read() {
        if let Ok(civilian) = civilians.get(event.entity)
            && civilian.kind == CivilianKind::Engineer
        {
            selected_engineer = Some(civilian);
        }
    }

    if let Some(_engineer) = selected_engineer {
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
                        BuildDepotButton,
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
                        BuildPortButton,
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

/// Handle button clicks in Engineer orders UI
pub fn handle_order_button_clicks(
    interactions: Query<
        (
            &Interaction,
            Option<&BuildDepotButton>,
            Option<&BuildPortButton>,
        ),
        Changed<Interaction>,
    >,
    selected_civilian: Query<(Entity, &Civilian), With<Civilian>>,
    mut order_writer: MessageWriter<GiveCivilianOrder>,
) {
    for (interaction, depot_button, port_button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find selected civilian
            if let Some((entity, _civilian)) = selected_civilian.iter().find(|(_, c)| c.selected) {
                if depot_button.is_some() {
                    info!("Build Depot button clicked for civilian {:?}", entity);
                    order_writer.write(GiveCivilianOrder {
                        entity,
                        order: CivilianOrderKind::BuildDepot,
                    });
                } else if port_button.is_some() {
                    info!("Build Port button clicked for civilian {:?}", entity);
                    order_writer.write(GiveCivilianOrder {
                        entity,
                        order: CivilianOrderKind::BuildPort,
                    });
                }
            }
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
    let mut selected_improver = None;
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
            selected_improver = Some(civilian);
        }
    }

    if let Some(improver) = selected_improver {
        // Resource improver is selected, ensure panel exists
        if existing_panel.is_empty() {
            let panel_title = format!("{:?} Orders", improver.kind);
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
                        ImproveTileButton,
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
    let mut selected_with_prev = None;
    for event in select_events.read() {
        if let Ok(prev_pos) = civilians_with_prev.get(event.entity) {
            selected_with_prev = Some(prev_pos);
        }
    }

    if let Some(prev_pos) = selected_with_prev {
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
                        RescindOrdersButton,
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

/// Handle button clicks in resource improver orders UI
pub fn handle_improver_button_clicks(
    interactions: Query<(&Interaction, &ImproveTileButton), Changed<Interaction>>,
    selected_civilian: Query<(Entity, &Civilian), With<Civilian>>,
    mut order_writer: MessageWriter<GiveCivilianOrder>,
) {
    for (interaction, _button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find selected civilian
            if let Some((entity, civilian)) = selected_civilian.iter().find(|(_, c)| c.selected) {
                info!("Improve Tile button clicked for {:?}", civilian.kind);
                order_writer.write(GiveCivilianOrder {
                    entity,
                    order: CivilianOrderKind::ImproveTile,
                });
            }
        }
    }
}

/// Handle button clicks in rescind orders UI
pub fn handle_rescind_button_clicks(
    interactions: Query<(&Interaction, &RescindOrdersButton), Changed<Interaction>>,
    selected_civilians: Query<(Entity, &Civilian, &PreviousPosition), With<Civilian>>,
    mut rescind_writer: MessageWriter<RescindOrders>,
) {
    for (interaction, _button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find selected civilian with previous position
            if let Some((entity, civilian, _prev)) =
                selected_civilians.iter().find(|(_, c, _)| c.selected)
            {
                info!(
                    "Rescind Orders button clicked for {:?} at ({}, {})",
                    civilian.kind, civilian.position.x, civilian.position.y
                );
                rescind_writer.write(RescindOrders { entity });
            }
        }
    }
}
