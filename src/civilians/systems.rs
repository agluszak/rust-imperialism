use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::civilians::commands::{
    DeselectAllCivilians, DeselectCivilian, RescindOrders, SelectCivilian,
};
use crate::civilians::order_validation::validate_command;
use crate::civilians::types::{
    ActionTurn, Civilian, CivilianJob, CivilianOrder, CivilianOrderKind, PreviousPosition,
};
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
    for _ in events.read() {
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
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    mut events: MessageReader<SelectCivilian>,
    mut civilians: Query<&mut Civilian>,
) {
    let Some(player) = player_nation else {
        return; // No player nation set yet
    };

    for event in events.read() {
        info!(
            "Processing SelectCivilian event for entity {:?}",
            event.entity
        );

        // Check ownership first - only allow selecting player-owned units
        let Ok(civilian_check) = civilians.get(event.entity) else {
            warn!("Failed to get civilian entity {:?}", event.entity);
            continue;
        };

        if civilian_check.owner != player.entity() {
            info!(
                "Ignoring selection of non-player unit {:?} (owner: {:?}, player: {:?})",
                event.entity,
                civilian_check.owner,
                player.entity()
            );
            continue;
        }

        // Check if clicking on already-selected unit (toggle deselect)
        let is_already_selected = civilian_check.selected;

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
    civilians: Query<(&Civilian, Option<&CivilianJob>, Option<&CivilianOrder>)>,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut rejection_writer: MessageWriter<CivilianCommandRejected>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    let tile_storage = tile_storage_query.iter().next();

    for command in events.read() {
        let (civilian, job, existing_order) = match civilians.get(command.civilian) {
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
            existing_order,
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

/// Execute SkipTurn and Sleep orders
pub fn execute_skip_and_sleep_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        match order.target {
            CivilianOrderKind::SkipTurn => {
                // Skip this turn only - remove order so they're available next turn
                civilian.has_moved = true;
                log_events.write(TerminalLogEvent {
                    message: format!(
                        "{:?} at ({}, {}) is skipping this turn",
                        civilian.kind, civilian.position.x, civilian.position.y
                    ),
                });
                commands.entity(entity).remove::<CivilianOrder>();
            }
            CivilianOrderKind::Sleep => {
                // Keep sleeping - mark as moved but keep order so it persists
                civilian.has_moved = true;
                // Don't remove order - it persists until rescinded
                // Note: No log message to avoid spam every turn
            }
            _ => {
                // Not a skip/sleep order, ignore
            }
        }
    }
}

/// Handle rescind orders - undo a civilian's action this turn
/// Uses exclusive world access to immediately remove components
pub fn handle_rescind_orders(world: &mut World) {
    // Use SystemState to read messages properly
    let mut events_to_process = Vec::new();
    {
        let mut state: bevy::ecs::system::SystemState<MessageReader<RescindOrders>> =
            bevy::ecs::system::SystemState::new(world);
        let mut rescind_reader = state.get_mut(world);
        for event in rescind_reader.read() {
            info!(
                "handle_rescind_orders: received rescind event for {:?}",
                event.entity
            );
            events_to_process.push(*event);
        }
        state.apply(world);
    }

    if events_to_process.is_empty() {
        return;
    }

    info!(
        "handle_rescind_orders: processing {} rescind events",
        events_to_process.len()
    );

    // Get turn system for refund logic
    let current_turn = world.resource::<TurnSystem>().current_turn;

    // Process each rescind event
    for event in events_to_process {
        let Ok(mut entity_mut) = world.get_entity_mut(event.entity) else {
            continue;
        };

        // Get required components (immutable first to avoid borrow conflicts)
        let Some(prev_pos) = entity_mut.get::<PreviousPosition>().copied() else {
            continue;
        };
        let action_turn_opt = entity_mut.get::<ActionTurn>().copied();
        let job_type_opt = entity_mut.get::<CivilianJob>().map(|j| j.job_type);

        // Now get mutable reference
        let Some(mut civilian) = entity_mut.get_mut::<Civilian>() else {
            continue;
        };

        let owner = civilian.owner;
        let old_pos = civilian.position;
        let kind = civilian.kind;

        // Determine if refund should be given (before removing components)
        let should_refund = action_turn_opt
            .map(|at| at.0 == current_turn)
            .unwrap_or(false);

        // Calculate refund amount
        let refund_amount = if should_refund {
            job_type_opt.and_then(|job_type| match job_type {
                crate::civilians::types::JobType::BuildingRail => Some(50),
                crate::civilians::types::JobType::BuildingDepot => Some(100),
                crate::civilians::types::JobType::BuildingPort => Some(150),
                _ => None,
            })
        } else {
            None
        };

        // Restore previous position
        civilian.position = prev_pos.0;
        civilian.has_moved = false;

        // Immediately remove components (exclusive world access = no queueing)
        entity_mut.remove::<CivilianJob>();
        entity_mut.remove::<CivilianOrder>();
        entity_mut.remove::<PreviousPosition>();
        entity_mut.remove::<ActionTurn>();

        // Apply refund and log
        let mut log_msg = String::new();
        if let Some(amount) = refund_amount {
            if let Some(mut treasury) = world.get_mut::<Treasury>(owner) {
                treasury.add(amount);
                log_msg = format!(
                    "{:?} orders rescinded - returned to ({}, {}) from ({}, {}). ${} refunded (same turn).",
                    kind, prev_pos.0.x, prev_pos.0.y, old_pos.x, old_pos.y, amount
                );
            }
        } else {
            let refund_note = if should_refund {
                "(no cost to refund)"
            } else {
                "(no refund - action was on a previous turn)"
            };
            log_msg = format!(
                "{:?} orders rescinded - returned to ({}, {}) from ({}, {}) {}",
                kind, prev_pos.0.x, prev_pos.0.y, old_pos.x, old_pos.y, refund_note
            );
        }

        // Write log event
        if !log_msg.is_empty() {
            let mut log_messages =
                world.resource_mut::<bevy::prelude::Messages<TerminalLogEvent>>();
            log_messages.write(TerminalLogEvent { message: log_msg });
        }
    }
}
