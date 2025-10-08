use bevy::prelude::*;

use super::commands::{GiveCivilianOrder, RescindOrders};
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

/// Show/hide Engineer orders UI based on selection
/// Only runs when Civilian selection state changes
pub fn update_engineer_orders_ui(
    mut commands: Commands,
    civilians: Query<&Civilian, Changed<Civilian>>,
    all_civilians: Query<&Civilian>,
    existing_panel: Query<(Entity, &Children), With<EngineerOrdersPanel>>,
) {
    // Only run if any Civilian changed (e.g., selection state)
    if civilians.is_empty() {
        return;
    }

    let selected_engineer = all_civilians
        .iter()
        .find(|c| c.selected && c.kind == CivilianKind::Engineer);

    if let Some(_engineer) = selected_engineer {
        // Engineer is selected, ensure panel exists
        if existing_panel.is_empty() {
            info!("Creating Engineer orders panel");
            commands
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
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                    EngineerOrdersPanel,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Engineer Orders"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ));

                    // Build Depot button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            BuildDepotButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Build Depot"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.95, 1.0)),
                            ));
                        });

                    // Build Port button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            BuildPortButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Build Port"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.95, 1.0)),
                            ));
                        });
                });
        }
    } else {
        // No engineer selected, remove panel and its children
        for (entity, children) in existing_panel.iter() {
            // Despawn all children first
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            // Then despawn the panel itself
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

/// Show/hide resource improver orders UI based on selection
/// Only runs when Civilian selection state changes
pub fn update_improver_orders_ui(
    mut commands: Commands,
    civilians: Query<&Civilian, Changed<Civilian>>,
    all_civilians: Query<&Civilian>,
    existing_panel: Query<(Entity, &Children), With<ImproverOrdersPanel>>,
) {
    // Only run if any Civilian changed (e.g., selection state)
    if civilians.is_empty() {
        return;
    }

    let selected_improver = all_civilians.iter().find(|c| {
        c.selected
            && matches!(
                c.kind,
                CivilianKind::Farmer
                    | CivilianKind::Rancher
                    | CivilianKind::Forester
                    | CivilianKind::Miner
                    | CivilianKind::Driller
            )
    });

    if let Some(improver) = selected_improver {
        // Resource improver is selected, ensure panel exists
        if existing_panel.is_empty() {
            let panel_title = format!("{:?} Orders", improver.kind);
            info!("Creating {} orders panel", panel_title);
            commands
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
                    ImproverOrdersPanel,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(panel_title),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ));

                    // Improve Tile button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            ImproveTileButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Improve Tile"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                            ));
                        });
                });
        }
    } else {
        // No improver selected, remove panel and its children
        for (entity, children) in existing_panel.iter() {
            // Despawn all children first
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            // Then despawn the panel itself
            commands.entity(entity).despawn();
        }
    }
}

/// Update UI for rescind orders panel (shown for any civilian with PreviousPosition)
pub fn update_rescind_orders_ui(
    mut commands: Commands,
    selected_civilians: Query<(Entity, &Civilian, &PreviousPosition), With<Civilian>>,
    existing_panel: Query<(Entity, &Children), With<RescindOrdersPanel>>,
) {
    // Check if there's a selected civilian with PreviousPosition
    let selected_with_prev = selected_civilians.iter().find(|(_, c, _)| c.selected);

    if let Some((_entity, _civilian, prev_pos)) = selected_with_prev {
        // Civilian is selected and has a previous position - show panel
        if existing_panel.is_empty() {
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
                    // Title showing previous position
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

                    // Rescind Orders button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_DANGER),
                            crate::ui::button_style::DangerButton,
                            RescindOrdersButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Rescind Orders"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.9, 0.9)),
                            ));
                        });

                    // Warning text about refund policy
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
    } else {
        // No selected civilian with previous position, remove panel and its children
        for (entity, children) in existing_panel.iter() {
            // Despawn all children first
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            // Then despawn the panel itself
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
