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
use crate::turn_system::TurnCounter;

/// Handle clicks on civilian visuals to select them
pub fn handle_civilian_click(
    trigger: On<Pointer<Click>>,
    mut commands: Commands,
    visuals: Query<&MapVisualFor>,
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
        commands.trigger(SelectCivilian {
            entity: visual_for.0,
        });
    }
}

/// Handle Escape key to deselect the selected civilian
pub fn handle_deselect_key(keys: Option<Res<ButtonInput<KeyCode>>>, mut commands: Commands) {
    if let Some(keys) = keys
        && keys.just_pressed(KeyCode::Escape)
    {
        commands.trigger(DeselectCivilian);
    }
}

/// Handle deselection event
pub fn handle_deselection(
    _trigger: On<DeselectCivilian>,
    mut commands: Commands,
    selected: Option<Res<SelectedCivilian>>,
) {
    if let Some(selected) = selected {
        info!("Deselected civilian {:?}", selected.0);
        commands.remove_resource::<SelectedCivilian>();
    }
}

/// Handle civilian selection events
pub fn handle_civilian_selection(
    trigger: On<SelectCivilian>,
    mut commands: Commands,
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    selected: Option<Res<SelectedCivilian>>,
    civilians: Query<&Civilian>,
) {
    let Some(player) = player_nation else {
        return; // No player nation set yet
    };

    let event = trigger.event();

    info!(
        "Processing SelectCivilian event for entity {:?}",
        event.entity
    );

    // Check ownership first - only allow selecting player-owned units
    let Ok(civilian_check) = civilians.get(event.entity) else {
        warn!("Failed to get civilian entity {:?}", event.entity);
        return;
    };

    if civilian_check.owner != player.entity() {
        warn!(
            "Attempted to select enemy civilian {:?} owned by {:?}",
            event.entity, civilian_check.owner
        );
        return;
    }

    // If this unit is already selected, do nothing
    if selected.map(|s| s.0) == Some(event.entity) {
        info!("Civilian {:?} is already selected", event.entity);
        return;
    }

    // Select the new civilian (automatically deselects any previously selected one)
    commands.insert_resource(SelectedCivilian(event.entity));
    info!("Selected civilian {:?}", event.entity);
}

/// Handle civilian command events and validate them before attaching orders
pub fn handle_civilian_commands(
    trigger: On<CivilianCommand>,
    mut commands: Commands,
    civilians: Query<(&Civilian, Option<&CivilianJob>, Option<&CivilianOrder>)>,
    all_civilians: Query<&Civilian>,
    tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
) {
    let command = trigger.event();
    let tile_data = tile_storage_query.iter().next();

    let (civilian, job, existing_order) = match civilians.get(command.civilian) {
        Ok(values) => values,
        Err(_) => {
            commands.trigger(CivilianCommandRejected {
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
            return;
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
        &all_civilians,
    ) {
        Ok(()) => {
            commands.entity(command.civilian).insert(CivilianOrder {
                target: command.order,
            });
        }
        Err(reason) => {
            commands.trigger(CivilianCommandRejected {
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

/// Execute Move orders for all civilian types
pub fn execute_move_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    turn: Res<TurnCounter>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        if let CivilianOrderKind::Move { to } = order.target {
            // Store previous position for potential undo
            let previous_pos = civilian.position;

            // Simple movement: just set position (TODO: implement pathfinding)
            civilian.position = to;
            civilian.has_moved = true;
            // Auto-deselect after moving
            commands.trigger(DeselectCivilian);

            // Add PreviousPosition and ActionTurn to allow rescinding
            commands
                .entity(entity)
                .insert((PreviousPosition(previous_pos), ActionTurn(turn.current)));

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
pub fn handle_rescind_orders(
    trigger: On<RescindOrders>,
    mut commands: Commands,
    turn: Res<TurnCounter>,
) {
    let event = trigger.event();
    let entity = event.entity;
    let current_turn = turn.current;

    info!(
        "handle_rescind_orders: received rescind event for {:?}",
        entity
    );

    // Perform updates in a command to ensure atomicity (all updates happen at sync point)
    // This prevents race conditions where position is reset but order/job still exists
    commands.queue(move |world: &mut World| {
        // Collect necessary data first (read-only)
        let (owner, old_pos, kind) = if let Some(civilian) = world.get::<Civilian>(entity) {
            (civilian.owner, civilian.position, civilian.kind)
        } else {
            return;
        };

        let prev_pos = if let Some(pp) = world.get::<PreviousPosition>(entity) {
            pp.0
        } else {
            return;
        };

        let action_turn = world.get::<ActionTurn>(entity).map(|at| at.0);
        let job_type = world.get::<CivilianJob>(entity).map(|j| j.job_type);

        // Determine refund
        let should_refund = action_turn.map(|at| at == current_turn).unwrap_or(false);

        let refund_amount = if should_refund {
            job_type.and_then(|t| match t {
                crate::civilians::types::JobType::BuildingRail => Some(50),
                crate::civilians::types::JobType::BuildingDepot => Some(100),
                crate::civilians::types::JobType::BuildingPort => Some(150),
                _ => None,
            })
        } else {
            None
        };

        // Mutate civilian state
        if let Some(mut civilian) = world.get_mut::<Civilian>(entity) {
            civilian.position = prev_pos;
            civilian.has_moved = false;
        }

        // Remove components
        world.entity_mut(entity)
            .remove::<CivilianJob>()
            .remove::<CivilianOrder>()
            .remove::<PreviousPosition>()
            .remove::<ActionTurn>();

        // Apply refund
        let mut log_msg = String::new();
        if let Some(amount) = refund_amount {
            if let Some(mut treasury) = world.get_mut::<Treasury>(owner) {
                treasury.add(amount);
                log_msg = format!(
                    "{:?} orders rescinded - returned to ({}, {}) from ({}, {}). ${} refunded (same turn).",
                    kind, prev_pos.x, prev_pos.y, old_pos.x, old_pos.y, amount
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
                kind, prev_pos.x, prev_pos.y, old_pos.x, old_pos.y, refund_note
            );
        }

        if !log_msg.is_empty() {
            info!("{}", log_msg);
        }
    });
}
