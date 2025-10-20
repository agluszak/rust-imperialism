use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button, observe};

use super::types::{
    BuildingDialog, CloseBuildingDialog, DialogCloseButton, DialogDragHandle, DialogDragState,
};

/// Spawn a dialog window frame (Rendering Layer)
/// Returns the entity ID of the dialog
pub fn spawn_dialog_frame(
    commands: &mut Commands,
    parent_entity: Entity,
    title: &str,
    building_entity: Entity,
    building_kind: crate::economy::production::BuildingKind,
    z_index: i32,
) -> Entity {
    let mut dialog_entity = Entity::PLACEHOLDER;
    let mut content_entity = Entity::PLACEHOLDER;
    let mut header_entity = Entity::PLACEHOLDER;

    commands.entity(parent_entity).with_children(|parent| {
        dialog_entity = parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    // Position dialogs in a cascading pattern based on z-index
                    left: Val::Px(300.0 + (z_index as f32 * 30.0)),
                    top: Val::Px(200.0 + (z_index as f32 * 30.0)),
                    width: Val::Px(380.0),
                    min_height: Val::Px(400.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.98)),
                BorderColor::all(Color::srgba(0.5, 0.6, 0.7, 1.0)),
                ZIndex(z_index),
                DialogDragState {
                    is_dragging: false,
                    drag_offset: Vec2::ZERO,
                },
                // BuildingDialog will be added after we know content_entity
            ))
            .with_children(|dialog| {
                // Header row: title + close button (draggable)
                header_entity =
                    dialog
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                            Interaction::None,
                            DialogDragHandle {
                                dialog_entity: Entity::PLACEHOLDER, // Will be updated below
                            },
                        ))
                        .with_children(|header| {
                            // Title
                            header.spawn((
                                Text::new(title),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.95, 0.8)),
                            ));

                            // Close button
                            header
                                .spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 1.0)),
                                    BorderColor::all(Color::srgba(0.7, 0.3, 0.3, 1.0)),
                                    DialogCloseButton { building_entity },
                                    observe(
                                        move |_: On<Activate>,
                                              mut close_writer: MessageWriter<
                                            CloseBuildingDialog,
                                        >| {
                                            close_writer
                                                .write(CloseBuildingDialog { building_entity });
                                        },
                                    ),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new("×"),
                                        TextFont {
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(1.0, 0.9, 0.9)),
                                    ));
                                });
                        })
                        .id();

                // Divider
                dialog.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::bottom(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.4, 0.5, 0.6, 0.5)),
                ));

                // Content area (will be populated by specific dialog types)
                content_entity = dialog
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        // Marker for dialog content area
                        super::types::DialogContentArea,
                    ))
                    .id();
            })
            .id();
    });

    // Now add the BuildingDialog component with the content_entity
    commands.entity(dialog_entity).insert(BuildingDialog {
        building_entity,
        building_kind,
        z_index,
        content_entity,
    });

    // Update the drag handle with the correct dialog entity
    commands
        .entity(header_entity)
        .insert(DialogDragHandle { dialog_entity });

    dialog_entity
}

/// Update close button visuals on hover (Rendering Layer)
pub fn update_close_button_visuals(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<DialogCloseButton>),
    >,
) {
    for (interaction, mut bg_color) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgba(0.7, 0.2, 0.2, 1.0));
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.6, 0.25, 0.25, 1.0));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 1.0));
            }
        }
    }
}
