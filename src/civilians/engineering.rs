use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::civilians::commands::DeselectCivilian;
use crate::civilians::order_validation::tile_owned_by_nation;
use crate::civilians::types::{
    ActionTurn, Civilian, CivilianJob, CivilianKind, CivilianOrder, CivilianOrderKind, JobType,
    PreviousPosition, ProspectingKnowledge,
};
use crate::economy::transport::{Rails, ordered_edge};
use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::map::province::{Province, TileProvince};
use crate::resources::{DevelopmentLevel, TileResource};
use crate::turn_system::TurnSystem;

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
            info!(
                "Engineer cannot act at ({}, {}): tile not owned by your nation",
                civilian.position.x, civilian.position.y
            );
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
        info!(
            "Cannot build rail to ({}, {}): tile not owned by your nation",
            to.x, to.y
        );
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
        info!(
            "Rail already exists between ({}, {}) and ({}, {}). Engineer moved.",
            edge.0.x, edge.0.y, edge.1.x, edge.1.y
        );
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
    potential_minerals: Query<&crate::map::PotentialMineral>,
    prospected_tiles: Query<(
        Option<&crate::map::ProspectedEmpty>,
        Option<&crate::map::ProspectedMineral>,
    )>,
) {
    for (entity, mut civilian, order) in prospectors.iter_mut() {
        // Only process Prospector units
        if civilian.kind != CivilianKind::Prospector {
            continue;
        }

        if let CivilianOrderKind::Prospect { to } = order.target {
            // Check territory ownership of target tile
            let has_territory_access = tile_storage_query
                .iter()
                .next()
                .map(|tile_storage| {
                    tile_owned_by_nation(
                        to,
                        civilian.owner,
                        tile_storage,
                        &tile_provinces,
                        &provinces,
                    )
                })
                .unwrap_or(false);

            if !has_territory_access {
                info!(
                    "Prospector cannot act at ({}, {}): tile not owned by your nation",
                    to.x, to.y
                );
                commands.entity(entity).remove::<CivilianOrder>();
                continue;
            }

            // Find tile entity and check if it can be prospected
            if let Some(tile_storage) = tile_storage_query.iter().next()
                && let Some(tile_entity) = tile_storage.get(&to)
            {
                // Check if tile has already been prospected
                if let Ok((empty, mineral)) = prospected_tiles.get(tile_entity)
                    && (empty.is_some() || mineral.is_some())
                {
                    info!("Tile at ({}, {}) has already been prospected", to.x, to.y);
                    commands.entity(entity).remove::<CivilianOrder>();
                    continue;
                }

                // Check if tile has potential mineral deposits
                if potential_minerals.get(tile_entity).is_ok() {
                    // Store previous position for potential undo
                    let previous_pos = civilian.position;

                    // Move to target tile
                    civilian.position = to;

                    let job_type = civilian
                        .kind
                        .order_definition(&order.target)
                        .and_then(|definition| definition.execution.job_type())
                        .unwrap_or(JobType::Prospecting);

                    commands.entity(entity).insert((
                        CivilianJob {
                            job_type,
                            turns_remaining: job_type.duration(),
                            target: to,
                        },
                        PreviousPosition(previous_pos),
                        ActionTurn(turn.current_turn),
                    ));

                    info!(
                        "Prospector moved to ({}, {}) and began surveying for minerals",
                        to.x, to.y
                    );
                    civilian.has_moved = true;
                    deselect_writer.write(DeselectCivilian { entity });
                } else {
                    info!(
                        "Cannot prospect at ({}, {}): no mineral deposits possible here",
                        to.x, to.y
                    );
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
    tile_resources: Query<&TileResource>,
    prospecting_knowledge: Res<ProspectingKnowledge>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        // Only process civilians that support tile improvements
        if !civilian.kind.supports_improvements() {
            continue;
        }

        let Some(resource_predicate) = civilian.kind.improvement_predicate() else {
            continue;
        };
        let Some(job_type) = civilian.kind.improvement_job() else {
            continue;
        };

        // Extract target position from order
        let target_pos = match order.target {
            CivilianOrderKind::ImproveTile { to }
            | CivilianOrderKind::Mine { to }
            | CivilianOrderKind::BuildFarm { to }
            | CivilianOrderKind::BuildOrchard { to } => to,
            _ => {
                // Not an improvement order
                continue;
            }
        };

        // Check territory ownership of target tile
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    target_pos,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            info!(
                "{:?} cannot act at ({}, {}): tile not owned by your nation",
                civilian.kind, target_pos.x, target_pos.y
            );
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        // Find tile entity and validate resource
        if let Some(tile_storage) = tile_storage_query.iter().next()
            && let Some(tile_entity) = tile_storage.get(&target_pos)
        {
            if let Ok(resource) = tile_resources.get(tile_entity) {
                if resource.requires_prospecting()
                    && !prospecting_knowledge.is_discovered_by(tile_entity, civilian.owner)
                {
                    info!(
                        "{:?} must have this tile prospected before improving it",
                        civilian.kind
                    );
                    commands.entity(entity).remove::<CivilianOrder>();
                    continue;
                }

                if !resource.discovered {
                    info!(
                        "{:?} must have this tile prospected before improving it",
                        civilian.kind
                    );
                    commands.entity(entity).remove::<CivilianOrder>();
                    continue;
                }

                let can_improve = resource_predicate(resource);

                if can_improve && resource.development < DevelopmentLevel::Lv3 {
                    // Store previous position for potential undo
                    let previous_pos = civilian.position;

                    // Move to target tile
                    civilian.position = target_pos;

                    // Start improvement job
                    commands.entity(entity).insert((
                        CivilianJob {
                            job_type,
                            turns_remaining: job_type.duration(),
                            target: target_pos,
                        },
                        PreviousPosition(previous_pos),
                        ActionTurn(turn.current_turn),
                    ));

                    info!(
                        "{:?} moved to ({}, {}) and started improving {:?} - {} turns remaining",
                        civilian.kind,
                        target_pos.x,
                        target_pos.y,
                        resource.resource_type,
                        job_type.duration()
                    );
                    civilian.has_moved = true;
                    deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                } else if !can_improve {
                    info!(
                        "{:?} cannot improve {:?} at ({}, {})",
                        civilian.kind, resource.resource_type, target_pos.x, target_pos.y
                    );
                } else if resource.development >= DevelopmentLevel::Lv3 {
                    info!(
                        "Resource already at max development at ({}, {})",
                        target_pos.x, target_pos.y
                    );
                }
            } else {
                info!(
                    "No improvable resource at ({}, {})",
                    target_pos.x, target_pos.y
                );
            }
        }

        commands.entity(entity).remove::<CivilianOrder>();
    }
}
