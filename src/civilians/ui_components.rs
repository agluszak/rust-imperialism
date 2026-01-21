use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button};

use crate::civilians::commands::{DeselectCivilian, RescindOrders, SelectCivilian};
use crate::civilians::types::{Civilian, CivilianOrderDefinition, PreviousPosition};
use crate::messages::civilians::CivilianCommand;
use crate::ui::button_style::*;

/// Marker for civilian orders UI panel
#[derive(Component)]
pub struct CivilianOrdersPanel;

/// Marker for rescind orders panel
#[derive(Component)]
pub struct RescindOrdersPanel;

/// Hide civilian orders UI on deselect
pub fn hide_civilian_orders_ui(
    _trigger: On<DeselectCivilian>,
    mut commands: Commands,
    existing_panel: Query<Entity, With<CivilianOrdersPanel>>,
) {
    for entity in existing_panel.iter() {
        commands.entity(entity).despawn();
    }
}

/// Show civilian orders UI on select
pub fn show_civilian_orders_ui(
    trigger: On<SelectCivilian>,
    mut commands: Commands,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    civilians: Query<&Civilian>,
    existing_panel: Query<Entity, With<CivilianOrdersPanel>>,
) {
    // Early exit if no player nation set
    let Some(player) = player_nation else {
        return;
    };

    // Remove existing panel first (just in case)
    for entity in existing_panel.iter() {
        commands.entity(entity).despawn();
    }

    let event = trigger.event();

    let Ok(civilian) = civilians.get(event.entity) else {
        return;
    };

    // Only show UI for player-owned units
    if civilian.owner != player.entity() {
        return;
    }

    let definition = civilian.kind.definition();
    if !definition.show_orders_panel || definition.orders.is_empty() {
        return;
    }

    let display_name = definition.display_name;
    let buttons = definition.orders;
    let civilian_entity = event.entity;

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
                          mut commands: Commands,
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

                        commands.trigger(CivilianCommand {
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

/// Hide rescind orders UI on deselect
pub fn hide_rescind_orders_ui(
    _trigger: On<DeselectCivilian>,
    mut commands: Commands,
    existing_panel: Query<Entity, With<RescindOrdersPanel>>,
) {
    for entity in existing_panel.iter() {
        commands.entity(entity).despawn();
    }
}

/// Show rescind orders UI on select
pub fn show_rescind_orders_ui(
    trigger: On<SelectCivilian>,
    mut commands: Commands,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    civilians_with_prev: Query<(&Civilian, &PreviousPosition)>,
    existing_panel: Query<Entity, With<RescindOrdersPanel>>,
) {
    // Early exit if no player nation set
    let Some(player) = player_nation else {
        return;
    };

    // Remove existing panel first
    for entity in existing_panel.iter() {
        commands.entity(entity).despawn();
    }

    let event = trigger.event();

    if let Ok((civilian, prev_pos)) = civilians_with_prev.get(event.entity) {
        // Only show UI for player-owned units
        if civilian.owner != player.entity() {
            return;
        }

        info!(
            "Civilian {:?} selected with PreviousPosition {:?}",
            event.entity, prev_pos
        );

        // Civilian is selected and has a previous position - show panel
        info!(
            "Creating rescind orders panel for civilian {:?}",
            event.entity
        );
        let civilian_entity = event.entity;
        let prev_pos = *prev_pos;

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
                              mut commands: Commands| {
                            info!("Rescind Orders button clicked for civilian {:?}", civilian_entity);
                            commands.trigger(RescindOrders {
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
}
