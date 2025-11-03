use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button};

use crate::civilians::commands::{
    DeselectAllCivilians, DeselectCivilian, RescindOrders, SelectCivilian,
};
use crate::civilians::types::{Civilian, CivilianOrderDefinition, PreviousPosition};
use crate::messages::civilians::CivilianCommand;
use crate::ui::button_style::*;

/// Marker for civilian orders UI panel
#[derive(Component)]
pub struct CivilianOrdersPanel;

/// Marker for rescind orders panel
#[derive(Component)]
pub struct RescindOrdersPanel;

/// Show/hide civilian orders UI based on selection messages using metadata-driven buttons
pub fn update_civilian_orders_ui(
    mut commands: Commands,
    mut select_events: MessageReader<SelectCivilian>,
    mut deselect_all_events: MessageReader<DeselectAllCivilians>,
    civilians: Query<&Civilian>,
    existing_panel: Query<Entity, With<CivilianOrdersPanel>>,
) {
    // Handle deselect-all first (always hides panel)
    if !deselect_all_events.is_empty() {
        deselect_all_events.clear();
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let mut selection_changed = false;
    let mut panel_request: Option<(Entity, &'static str, &'static [CivilianOrderDefinition])> =
        None;

    for event in select_events.read() {
        selection_changed = true;

        if let Ok(civilian) = civilians.get(event.entity) {
            let definition = civilian.kind.definition();
            if definition.show_orders_panel && !definition.orders.is_empty() {
                panel_request = Some((event.entity, definition.display_name, definition.orders));
            } else {
                panel_request = None; // Selected unit has no actionable buttons
            }
        }
    }

    if selection_changed {
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
    }

    if let Some((civilian_entity, display_name, buttons)) = panel_request {
        let panel_entity = commands
            .spawn((
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
                CivilianOrdersPanel,
            ))
            .id();

        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new(format!("{} Orders", display_name)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.8)),
            ));

            for button in buttons {
                let order_kind = button.order;
                let label = button.label;

                parent
                    .spawn((
                        Button,
                        OldButton,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .observe(
                        move |_: On<Activate>,
                              mut order_writer: MessageWriter<CivilianCommand>,
                              civilians: Query<&Civilian>| {
                            // Get the civilian's current position to use as the target
                            let target_pos = civilians
                                .get(civilian_entity)
                                .map(|c| c.position)
                                .unwrap_or(bevy_ecs_tilemap::prelude::TilePos { x: 0, y: 0 });

                            // Update order coordinates with actual target position
                            use crate::civilians::types::CivilianOrderKind;
                            let actual_order = match order_kind {
                                CivilianOrderKind::Prospect { .. } => {
                                    CivilianOrderKind::Prospect { to: target_pos }
                                }
                                CivilianOrderKind::Mine { .. } => {
                                    CivilianOrderKind::Mine { to: target_pos }
                                }
                                CivilianOrderKind::ImproveTile { .. } => {
                                    CivilianOrderKind::ImproveTile { to: target_pos }
                                }
                                CivilianOrderKind::BuildFarm { .. } => {
                                    CivilianOrderKind::BuildFarm { to: target_pos }
                                }
                                CivilianOrderKind::BuildOrchard { .. } => {
                                    CivilianOrderKind::BuildOrchard { to: target_pos }
                                }
                                other => other, // Orders without coordinates remain unchanged
                            };

                            order_writer.write(CivilianCommand {
                                civilian: civilian_entity,
                                order: actual_order,
                            });
                        },
                    )
                    .with_children(|button_parent| {
                        button_parent.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        ));
                    });
            }
        });
    }
}

/// Update UI for rescind orders panel based on selection messages
/// Event-driven system that only runs when selection actually changes
pub fn update_rescind_orders_ui(
    mut commands: Commands,
    mut select_events: MessageReader<SelectCivilian>,
    mut deselect_events: MessageReader<DeselectCivilian>,
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

    // Handle individual deselect events (hide panel)
    if !deselect_events.is_empty() {
        deselect_events.clear();
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
        info!(
            "Civilian {:?} selected with PreviousPosition {:?}",
            civilian_entity, prev_pos
        );
        // Civilian is selected and has a previous position - show panel
        if existing_panel.is_empty() {
            info!(
                "Creating rescind orders panel for civilian {:?}",
                civilian_entity
            );
            // Create panel if it doesn't exist
            commands
                .spawn((
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
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!(
                            "Undo Action\nWas at: ({}, {})",
                            prev_pos.0.x, prev_pos.0.y
                        )),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.9, 0.7)),
                    ));

                    parent
                        .spawn((
                            Button,
                            OldButton,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_DANGER),
                            crate::ui::button_style::DangerButton,
                        ))
                        .observe(
                            move |_: On<Activate>,
                                  mut rescind_writer: MessageWriter<RescindOrders>| {
                                info!("Rescind Orders button clicked for civilian {:?}", civilian_entity);
                                rescind_writer.write(RescindOrders {
                                    entity: civilian_entity,
                                });
                            },
                        )
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new("Rescind Orders"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.9, 0.9)),
                            ));
                        });

                    parent.spawn((
                        Text::new("(Refund if same turn)"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.9, 0.7)),
                    ));
                });
        }
    } else if !select_events.is_empty() {
        // Selected civilian without previous position, remove panel if it exists
        for entity in existing_panel.iter() {
            commands.entity(entity).despawn();
        }
    }
}
