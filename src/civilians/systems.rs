use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use super::commands::{DeselectAllCivilians, DeselectCivilian, RescindOrders, SelectCivilian};
use super::types::{
    ActionTurn, Civilian, CivilianJob, CivilianOrder, CivilianOrderKind, PreviousPosition,
};
use crate::civilians::order_validation::validate_command;
use crate::economy::treasury::Treasury;
use crate::map::province::{Province, TileProvince};
use crate::map::rendering::MapVisualFor;
use crate::messages::civilians::{CivilianCommand, CivilianCommandError, CivilianCommandRejected};
use crate::turn_system::TurnSystem;
use crate::ui::logging::TerminalLogEvent;

/// Handle clicks on civilian visuals to select them
pub fn handle_civilian_click(
    trigger: On<Pointer<Click>>,
    visuals: Query<&MapVisualFor>,
    mut writer: MessageWriter<SelectCivilian>,
) {
    info!(
        "handle_civilian_click triggered for entity {:?}",
        trigger.entity
    );
    if let Ok(visual_for) = visuals.get(trigger.entity) {
        info!(
            "Sending SelectCivilian message for entity {:?}",
            visual_for.0
        );
        writer.write(SelectCivilian {
            entity: visual_for.0,
        });
    }
}

/// Handle Escape key to deselect all civilians
pub fn handle_deselect_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<DeselectAllCivilians>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        writer.write(DeselectAllCivilians);
    }
}

/// Handle deselection of specific civilians
pub fn handle_deselection(
    mut events: MessageReader<DeselectCivilian>,
    mut civilians: Query<&mut Civilian>,
) {
    for event in events.read() {
        if let Ok(mut civilian) = civilians.get_mut(event.entity) {
            civilian.selected = false;
            info!("Deselected civilian {:?}", event.entity);
        }
    }
}

/// Handle deselect-all events
pub fn handle_deselect_all(
    mut events: MessageReader<DeselectAllCivilians>,
    mut civilians: Query<&mut Civilian>,
) {
    if !events.is_empty() {
        events.clear();
        for mut civilian in civilians.iter_mut() {
            if civilian.selected {
                civilian.selected = false;
            }
        }
        info!("Deselected all civilians via Escape key");
    }
}

/// Handle civilian selection events
pub fn handle_civilian_selection(
    mut events: MessageReader<SelectCivilian>,
    mut civilians: Query<&mut Civilian>,
) {
    let event_list: Vec<_> = events.read().collect();

    if !event_list.is_empty() {
        info!(
            "handle_civilian_selection: received {} events",
            event_list.len()
        );
    }

    // Only process if there are events
    for event in event_list {
        info!(
            "Processing SelectCivilian event for entity {:?}",
            event.entity
        );

        // Check if clicking on already-selected unit (toggle deselect)
        let is_already_selected = civilians
            .get(event.entity)
            .map(|c| c.selected)
            .unwrap_or(false);

        if is_already_selected {
            // Deselect the unit (toggle off)
            if let Ok(mut civilian) = civilians.get_mut(event.entity) {
                civilian.selected = false;
                info!("Toggled deselect for entity {:?}", event.entity);
            }
        } else {
            // Deselect all units first
            for mut civilian in civilians.iter_mut() {
                civilian.selected = false;
            }

            // Select the requested unit
            if let Ok(mut civilian) = civilians.get_mut(event.entity) {
                civilian.selected = true;
                info!(
                    "Successfully set civilian.selected = true for entity {:?}",
                    event.entity
                );
            } else {
                warn!("Failed to get civilian entity {:?}", event.entity);
            }
        }
    }
}

/// Handle civilian command events and validate them before attaching orders
pub fn handle_civilian_commands(
    mut commands: Commands,
    mut events: MessageReader<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianJob>)>,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut rejection_writer: MessageWriter<CivilianCommandRejected>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    let tile_storage = tile_storage_query.iter().next();

    for command in events.read() {
        let (civilian, job) = match civilians.get(command.civilian) {
            Ok(values) => values,
            Err(_) => {
                rejection_writer.write(CivilianCommandRejected {
                    civilian: command.civilian,
                    order: command.order,
                    reason: CivilianCommandError::MissingCivilian,
                });
                log_rejection(
                    &mut log_events,
                    None,
                    command.civilian,
                    command.order,
                    CivilianCommandError::MissingCivilian,
                );
                continue;
            }
        };

        match validate_command(
            civilian,
            job,
            &command.order,
            tile_storage,
            &tile_provinces,
            &provinces,
        ) {
            Ok(()) => {
                commands.entity(command.civilian).insert(CivilianOrder {
                    target: command.order,
                });
            }
            Err(reason) => {
                rejection_writer.write(CivilianCommandRejected {
                    civilian: command.civilian,
                    order: command.order,
                    reason,
                });
                log_rejection(
                    &mut log_events,
                    Some((command.civilian, civilian)),
                    command.civilian,
                    command.order,
                    reason,
                );
            }
        }
    }
}

fn log_rejection(
    log_events: &mut MessageWriter<TerminalLogEvent>,
    civilian_data: Option<(Entity, &Civilian)>,
    civilian_entity: Entity,
    order: CivilianOrderKind,
    reason: CivilianCommandError,
) {
    let message = match (civilian_data, reason) {
        (Some((_, civilian)), CivilianCommandError::MissingTargetTile(pos)) => format!(
            "{:?} at ({}, {}) order {:?} rejected: target tile ({}, {}) not found",
            civilian.kind, civilian.position.x, civilian.position.y, order, pos.x, pos.y
        ),
        (Some((_, civilian)), other) => format!(
            "{:?} at ({}, {}) order {:?} rejected: {}",
            civilian.kind,
            civilian.position.x,
            civilian.position.y,
            order,
            other.describe()
        ),
        (None, CivilianCommandError::MissingTargetTile(pos)) => format!(
            "Order {:?} for {:?} rejected: target tile ({}, {}) not found",
            order, civilian_entity, pos.x, pos.y
        ),
        (None, other) => format!(
            "Order {:?} for {:?} rejected: {}",
            order,
            civilian_entity,
            other.describe()
        ),
    };

    log_events.write(TerminalLogEvent { message });
}

/// Execute Move orders for all civilian types
pub fn execute_move_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    turn: Res<TurnSystem>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        if let CivilianOrderKind::Move { to } = order.target {
            // Store previous position for potential undo
            let previous_pos = civilian.position;

            // Simple movement: just set position (TODO: implement pathfinding)
            civilian.position = to;
            civilian.has_moved = true;
            deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after moving

            // Add PreviousPosition and ActionTurn to allow rescinding
            commands.entity(entity).insert((
                PreviousPosition(previous_pos),
                ActionTurn(turn.current_turn),
            ));

            log_events.write(TerminalLogEvent {
                message: format!("{:?} moved to ({}, {})", civilian.kind, to.x, to.y),
            });

            commands.entity(entity).remove::<CivilianOrder>();
        }
    }
}

/// Handle rescind orders - undo a civilian's action this turn
pub fn handle_rescind_orders(
    mut commands: Commands,
    mut rescind_events: MessageReader<RescindOrders>,
    mut civilians: Query<(
        &mut Civilian,
        &PreviousPosition,
        Option<&ActionTurn>,
        Option<&CivilianJob>,
    )>,
    turn: Res<TurnSystem>,
    mut treasuries: Query<&mut Treasury>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for event in rescind_events.read() {
        if let Ok((mut civilian, prev_pos, action_turn_opt, job_opt)) =
            civilians.get_mut(event.entity)
        {
            let old_pos = civilian.position;

            // Restore previous position
            civilian.position = prev_pos.0;
            civilian.has_moved = false;

            // Determine if refund should be given (only if rescinding on the same turn)
            let should_refund = action_turn_opt
                .map(|at| at.0 == turn.current_turn)
                .unwrap_or(false);

            // Calculate refund amount based on job type
            let refund_amount = if should_refund {
                job_opt.and_then(|job| match job.job_type {
                    super::types::JobType::BuildingRail => Some(50),
                    super::types::JobType::BuildingDepot => Some(100),
                    super::types::JobType::BuildingPort => Some(150),
                    _ => None, // Other job types don't have direct costs
                })
            } else {
                None
            };

            // Apply refund if applicable
            if let Some(amount) = refund_amount {
                if let Ok(mut treasury) = treasuries.get_mut(civilian.owner) {
                    treasury.add(amount);
                    log_events.write(TerminalLogEvent {
                        message: format!(
                            "{:?} orders rescinded - returned to ({}, {}) from ({}, {}). ${} refunded (same turn).",
                            civilian.kind,
                            prev_pos.0.x, prev_pos.0.y,
                            old_pos.x, old_pos.y,
                            amount
                        ),
                    });
                }
            } else {
                let refund_msg = if should_refund {
                    "(no cost to refund)"
                } else {
                    "(no refund - action was on a previous turn)"
                };
                log_events.write(TerminalLogEvent {
                    message: format!(
                        "{:?} orders rescinded - returned to ({}, {}) from ({}, {}) {}",
                        civilian.kind, prev_pos.0.x, prev_pos.0.y, old_pos.x, old_pos.y, refund_msg
                    ),
                });
            }

            // Remove job and action tracking components
            commands
                .entity(event.entity)
                .remove::<CivilianJob>()
                .remove::<PreviousPosition>()
                .remove::<ActionTurn>();
        }
    }
}
