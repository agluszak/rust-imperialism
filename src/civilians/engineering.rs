use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use super::commands::DeselectCivilian;
use super::systems::tile_owned_by_nation;
use super::types::{
    ActionTurn, Civilian, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind, JobType,
    PreviousPosition,
};
use crate::economy::transport::{Rails, ordered_edge};
use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::province::{Province, TileProvince};
use crate::resources::{DevelopmentLevel, TileResource};
use crate::turn_system::TurnSystem;
use crate::ui::logging::TerminalLogEvent;

/// Execute Engineer orders (building infrastructure)
pub fn execute_engineer_orders(
    mut commands: Commands,
    mut engineers: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut improvement_writer: MessageWriter<PlaceImprovement>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    rails: Res<Rails>,
    turn: Res<TurnSystem>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in engineers.iter_mut() {
        // Only process Engineer units
        if civilian.kind != CivilianKind::Engineer {
            continue;
        }

        // Check territory ownership for all operations
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Engineer cannot act at ({}, {}): tile not owned by your nation",
                    civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        match order.target {
            CivilianOrderKind::BuildRail { to } => {
                handle_build_rail_order(
                    &mut commands,
                    entity,
                    &mut civilian,
                    to,
                    &mut improvement_writer,
                    &mut deselect_writer,
                    &rails,
                    &turn,
                    &tile_storage_query,
                    &tile_provinces,
                    &provinces,
                    &mut log_events,
                );
            }
            CivilianOrderKind::BuildDepot => {
                handle_build_depot_order(
                    &mut commands,
                    entity,
                    &mut civilian,
                    &mut improvement_writer,
                    &mut deselect_writer,
                    &turn,
                );
            }
            CivilianOrderKind::BuildPort => {
                handle_build_port_order(
                    &mut commands,
                    entity,
                    &mut civilian,
                    &mut improvement_writer,
                    &mut deselect_writer,
                    &turn,
                );
            }
            CivilianOrderKind::Move { .. } => {
                // Move orders are handled by execute_move_orders for all civilians
            }
            _ => {
                // Other order types handled by other systems
            }
        }

        // Remove order after execution
        commands.entity(entity).remove::<CivilianOrder>();
    }
}

fn handle_build_rail_order(
    commands: &mut Commands,
    entity: Entity,
    civilian: &mut Civilian,
    to: TilePos,
    improvement_writer: &mut MessageWriter<PlaceImprovement>,
    deselect_writer: &mut MessageWriter<DeselectCivilian>,
    rails: &Res<Rails>,
    turn: &Res<TurnSystem>,
    tile_storage_query: &Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    log_events: &mut MessageWriter<TerminalLogEvent>,
) {
    // Also check target tile ownership
    let target_owned = tile_storage_query
        .iter()
        .next()
        .map(|tile_storage| {
            tile_owned_by_nation(to, civilian.owner, tile_storage, tile_provinces, provinces)
        })
        .unwrap_or(false);

    if !target_owned {
        log_events.write(TerminalLogEvent {
            message: format!(
                "Cannot build rail to ({}, {}): tile not owned by your nation",
                to.x, to.y
            ),
        });
        commands.entity(entity).remove::<CivilianOrder>();
        return;
    }

    // Check if rail already exists
    let edge = ordered_edge(civilian.position, to);
    let rail_exists = rails.0.contains(&edge);

    // Store previous position for potential undo
    let previous_pos = civilian.position;

    if rail_exists {
        // Rail already exists - just move the engineer without starting a job
        log_events.write(TerminalLogEvent {
            message: format!(
                "Rail already exists between ({}, {}) and ({}, {}). Engineer moved.",
                edge.0.x, edge.0.y, edge.1.x, edge.1.y
            ),
        });
        civilian.position = to;
        civilian.has_moved = true;
        deselect_writer.write(DeselectCivilian { entity });
        commands.entity(entity).insert((
            PreviousPosition(previous_pos),
            ActionTurn(turn.current_turn),
        ));
    } else {
        // Rail doesn't exist - start construction
        // Send PlaceImprovement message with engineer entity
        improvement_writer.write(PlaceImprovement {
            a: civilian.position,
            b: to,
            kind: ImprovementKind::Rail,
            engineer: Some(entity),
        });
        // Move Engineer to the target tile after starting construction
        civilian.position = to;
        civilian.has_moved = true;
        deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
        // Add job to lock Engineer and previous position for rescinding
        let job_type = JobType::BuildingRail;
        commands.entity(entity).insert((
            CivilianJob {
                job_type,
                turns_remaining: job_type.duration(),
                target: to,
            },
            PreviousPosition(previous_pos),
            ActionTurn(turn.current_turn),
        ));
    }
}

fn handle_build_depot_order(
    commands: &mut Commands,
    entity: Entity,
    civilian: &mut Civilian,
    improvement_writer: &mut MessageWriter<PlaceImprovement>,
    deselect_writer: &mut MessageWriter<DeselectCivilian>,
    turn: &Res<TurnSystem>,
) {
    // Store previous position for potential undo
    let previous_pos = civilian.position;

    improvement_writer.write(PlaceImprovement {
        a: civilian.position,
        b: civilian.position, // Depot is single-tile
        kind: ImprovementKind::Depot,
        engineer: Some(entity),
    });
    civilian.has_moved = true;
    deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
    // Add job to lock Engineer and previous position for rescinding
    let job_type = JobType::BuildingDepot;
    commands.entity(entity).insert((
        CivilianJob {
            job_type,
            turns_remaining: job_type.duration(),
            target: civilian.position,
        },
        PreviousPosition(previous_pos),
        ActionTurn(turn.current_turn),
    ));
}

fn handle_build_port_order(
    commands: &mut Commands,
    entity: Entity,
    civilian: &mut Civilian,
    improvement_writer: &mut MessageWriter<PlaceImprovement>,
    deselect_writer: &mut MessageWriter<DeselectCivilian>,
    turn: &Res<TurnSystem>,
) {
    // Store previous position for potential undo
    let previous_pos = civilian.position;

    improvement_writer.write(PlaceImprovement {
        a: civilian.position,
        b: civilian.position,
        kind: ImprovementKind::Port,
        engineer: Some(entity),
    });
    civilian.has_moved = true;
    deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
    // Add job to lock Engineer and previous position for rescinding
    let job_type = JobType::BuildingPort;
    commands.entity(entity).insert((
        CivilianJob {
            job_type,
            turns_remaining: job_type.duration(),
            target: civilian.position,
        },
        PreviousPosition(previous_pos),
        ActionTurn(turn.current_turn),
    ));
}

/// Execute Prospector orders (mineral discovery)
pub fn execute_prospector_orders(
    mut commands: Commands,
    mut prospectors: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    turn: Res<TurnSystem>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut tile_resources: Query<&mut TileResource>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in prospectors.iter_mut() {
        // Only process Prospector units
        if civilian.kind != CivilianKind::Prospector {
            continue;
        }

        // Check territory ownership
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "Prospector cannot act at ({}, {}): tile not owned by your nation",
                    civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        if let CivilianOrderKind::Prospect = order.target {
            // Find tile entity and check for hidden mineral
            if let Some(tile_storage) = tile_storage_query.iter().next()
                && let Some(tile_entity) = tile_storage.get(&civilian.position)
            {
                if let Ok(mut resource) = tile_resources.get_mut(tile_entity) {
                    if !resource.discovered {
                        // Store previous position for potential undo
                        let previous_pos = civilian.position;

                        resource.discovered = true;
                        log_events.write(TerminalLogEvent {
                            message: format!(
                                "Prospector discovered {:?} at ({}, {})!",
                                resource.resource_type, civilian.position.x, civilian.position.y
                            ),
                        });
                        civilian.has_moved = true;
                        deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                        commands.entity(entity).insert((
                            PreviousPosition(previous_pos),
                            ActionTurn(turn.current_turn),
                        ));
                    } else {
                        log_events.write(TerminalLogEvent {
                            message: format!(
                                "No hidden minerals at ({}, {})",
                                civilian.position.x, civilian.position.y
                            ),
                        });
                    }
                } else {
                    log_events.write(TerminalLogEvent {
                        message: format!(
                            "No mineral deposits at ({}, {})",
                            civilian.position.x, civilian.position.y
                        ),
                    });
                }
            }
        }

        commands.entity(entity).remove::<CivilianOrder>();
    }
}

/// Execute Farmer/Rancher/Forester/Driller orders (resource improvement)
pub fn execute_civilian_improvement_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    turn: Res<TurnSystem>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    tile_resources: Query<&mut TileResource>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        // Check if this is a resource-improving civilian
        let is_improver = matches!(
            civilian.kind,
            CivilianKind::Farmer
                | CivilianKind::Rancher
                | CivilianKind::Forester
                | CivilianKind::Driller
                | CivilianKind::Miner
        );

        if !is_improver {
            continue;
        }

        // Check territory ownership
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "{:?} cannot act at ({}, {}): tile not owned by your nation",
                    civilian.kind, civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        if let CivilianOrderKind::ImproveTile = order.target {
            // Find tile entity and validate resource
            if let Some(tile_storage) = tile_storage_query.iter().next()
                && let Some(tile_entity) = tile_storage.get(&civilian.position)
            {
                if let Ok(resource) = tile_resources.get(tile_entity) {
                    // Check if this civilian can improve this resource
                    let can_improve = match civilian.kind {
                        CivilianKind::Farmer => resource.improvable_by_farmer(),
                        CivilianKind::Rancher => resource.improvable_by_rancher(),
                        CivilianKind::Forester => resource.improvable_by_forester(),
                        CivilianKind::Miner => resource.improvable_by_miner(),
                        CivilianKind::Driller => resource.improvable_by_driller(),
                        _ => false,
                    };

                    if can_improve && resource.development < DevelopmentLevel::Lv3 {
                        // Store previous position for potential undo
                        let previous_pos = civilian.position;

                        // Start improvement job
                        let job_type = JobType::ImprovingTile;
                        commands.entity(entity).insert((
                            CivilianJob {
                                job_type,
                                turns_remaining: job_type.duration(),
                                target: civilian.position,
                            },
                            PreviousPosition(previous_pos),
                            ActionTurn(turn.current_turn),
                        ));

                        log_events.write(TerminalLogEvent {
                            message: format!(
                                "{:?} started improving {:?} at ({}, {}) - {} turns remaining",
                                civilian.kind,
                                resource.resource_type,
                                civilian.position.x,
                                civilian.position.y,
                                job_type.duration()
                            ),
                        });
                        civilian.has_moved = true;
                        deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                    } else if resource.development >= DevelopmentLevel::Lv3 {
                        log_events.write(TerminalLogEvent {
                            message: format!(
                                "Resource already at max development at ({}, {})",
                                civilian.position.x, civilian.position.y
                            ),
                        });
                    } else {
                        log_events.write(TerminalLogEvent {
                            message: format!(
                                "{:?} cannot improve {:?}",
                                civilian.kind, resource.resource_type
                            ),
                        });
                    }
                } else {
                    log_events.write(TerminalLogEvent {
                        message: format!(
                            "No improvable resource at ({}, {})",
                            civilian.position.x, civilian.position.y
                        ),
                    });
                }
            }
        }

        commands.entity(entity).remove::<CivilianOrder>();
    }
}
