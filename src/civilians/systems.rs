use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TileStorage, TilemapSize};

use crate::civilians::commands::{
    DeselectCivilian, RescindOrders, SelectCivilian, SelectedCivilian,
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

/// Handle Escape key to deselect the selected civilian
pub fn handle_deselect_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut writer: MessageWriter<DeselectCivilian>,
) {
    if let Some(keys) = keys {
        if keys.just_pressed(KeyCode::Escape) {
            writer.write(DeselectCivilian);
        }
    }
}

/// Handle deselection event
pub fn handle_deselection(
    mut events: MessageReader<DeselectCivilian>,
    mut selected: ResMut<SelectedCivilian>,
) {
    for _ in events.read() {
        if let Some(entity) = selected.0 {
            info!("Deselected civilian {:?}", entity);
            selected.0 = None;
        }
    }
}

/// Handle civilian selection events
pub fn handle_civilian_selection(
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    mut events: MessageReader<SelectCivilian>,
    mut selected: ResMut<SelectedCivilian>,
    civilians: Query<&Civilian>,
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
            warn!(
                "Attempted to select enemy civilian {:?} owned by {:?}",
                event.entity, civilian_check.owner
            );
            continue;
        }

        // If this unit is already selected, do nothing
        if selected.0 == Some(event.entity) {
            info!("Civilian {:?} is already selected", event.entity);
            continue;
        }

        // Select the new civilian (automatically deselects any previously selected one)
        selected.0 = Some(event.entity);
        info!("Selected civilian {:?}", event.entity);
    }
}

/// Handle civilian command events and validate them before attaching orders
pub fn handle_civilian_commands(
    mut commands: Commands,
    mut events: MessageReader<CivilianCommand>,
    civilians: Query<(&Civilian, Option<&CivilianJob>, Option<&CivilianOrder>)>,
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut rejection_writer: MessageWriter<CivilianCommandRejected>,
) {
    let tile_data = tile_storage_query.iter().next();

    for command in events.read() {
        let (civilian, job, existing_order) = match civilians.get(command.civilian) {
            Ok(values) => values,
            Err(_) => {
                rejection_writer.write(CivilianCommandRejected {
                    civilian: command.civilian,
                    order: command.order,
                    reason: CivilianCommandError::MissingCivilian,
                });
                info!(
                    "Order {:?} for {:?} rejected: {}",
                    command.order,
                    command.civilian,
                    CivilianCommandError::MissingCivilian.describe()
                );
                continue;
            }
        };

        let (tile_storage, map_size) = match tile_data {
            Some((storage, size)) => (Some(storage), *size),
            None => (None, TilemapSize { x: 0, y: 0 }),
        };

        match validate_command(
            civilian,
            job,
            existing_order,
            &command.order,
            tile_storage,
            map_size,
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
                if let CivilianCommandError::MissingTargetTile(pos) = reason {
                    info!(
                        "{:?} at ({}, {}) order {:?} rejected: target tile ({}, {}) not found",
                        civilian.kind,
                        civilian.position.x,
                        civilian.position.y,
                        command.order,
                        pos.x,
                        pos.y
                    );
                } else {
                    info!(
                        "{:?} at ({}, {}) order {:?} rejected: {}",
                        civilian.kind,
                        civilian.position.x,
                        civilian.position.y,
                        command.order,
                        reason.describe()
                    );
                }
            }
        }
    }
}

/// Execute Move orders for all civilian types
pub fn execute_move_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    turn: Res<TurnSystem>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        if let CivilianOrderKind::Move { to } = order.target {
            // Store previous position for potential undo
            let previous_pos = civilian.position;

            // Simple movement: just set position (TODO: implement pathfinding)
            civilian.position = to;
            civilian.has_moved = true;
            // Auto-deselect after moving (note: DeselectCivilian has no effect if no civilian is selected)
            deselect_writer.write(DeselectCivilian);

            // Add PreviousPosition and ActionTurn to allow rescinding
            commands.entity(entity).insert((
                PreviousPosition(previous_pos),
                ActionTurn(turn.current_turn),
            ));

            info!(
                "{:?} (owner: {:?}) moved from ({}, {}) to ({}, {})",
                civilian.kind, civilian.owner, previous_pos.x, previous_pos.y, to.x, to.y
            );

            commands.entity(entity).remove::<CivilianOrder>();
        }
    }
}

/// Execute SkipTurn and Sleep orders
pub fn execute_skip_and_sleep_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        match order.target {
            CivilianOrderKind::SkipTurn => {
                // Skip this turn only - remove order so they're available next turn
                civilian.has_moved = true;
                info!(
                    "{:?} at ({}, {}) is skipping this turn",
                    civilian.kind, civilian.position.x, civilian.position.y
                );
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

        // Log the message
        if !log_msg.is_empty() {
            info!("{}", log_msg);
        }
    }
}
