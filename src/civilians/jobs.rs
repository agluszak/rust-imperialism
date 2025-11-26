use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::civilians::types::{
    ActionTurn, Civilian, CivilianJob, JobType, PreviousPosition, ProspectingKnowledge,
};
use crate::resources::TileResource;
use crate::turn_system::TurnCounter;

/// Reset civilian movement at start of player turn.
///
/// Note: Runs via OnEnter(TurnPhase::PlayerTurn) in CivilianJobSet::Reset.
pub fn reset_civilian_actions(mut civilians: Query<&mut Civilian>) {
    for mut civilian in civilians.iter_mut() {
        civilian.has_moved = false;
    }
}

/// Advance civilian jobs each turn.
///
/// Note: Runs via OnEnter(TurnPhase::PlayerTurn) in CivilianJobSet::Advance.
/// OnEnter fires exactly once per state entry, so no double-processing guard needed.
pub fn advance_civilian_jobs(
    turn: Res<TurnCounter>,
    mut civilians_with_jobs: Query<(Entity, &Civilian, &mut CivilianJob)>,
) {
    let count = civilians_with_jobs.iter().count();
    info!(
        "advance_civilian_jobs: turn {}, found {} civilians with jobs",
        turn.current, count
    );

    for (_entity, civilian, mut job) in civilians_with_jobs.iter_mut() {
        job.turns_remaining -= 1;

        if job.turns_remaining == 0 {
            info!(
                "{:?} (owner: {:?}) completed job {:?} - awaiting completion processing",
                civilian.kind, civilian.owner, job.job_type
            );
            // Don't remove CivilianJob here - complete_improvement_jobs needs to see it
            // to apply the actual improvement effect. It will handle the removal.
        } else {
            info!(
                "{:?} (owner: {:?}) job {:?} in progress: {} turns remaining",
                civilian.kind, civilian.owner, job.job_type, job.turns_remaining
            );
        }
    }
}

/// Complete improvement jobs when they finish
pub fn complete_improvement_jobs(
    mut commands: Commands,
    mut civilians_with_jobs: Query<(Entity, &Civilian, &mut CivilianJob)>,
    tile_storage_query: Query<&TileStorage>,
    mut tile_resources: Query<&mut TileResource>,
    potential_minerals: Query<&crate::map::PotentialMineral>,
    mut prospecting_knowledge: ResMut<ProspectingKnowledge>,
) {
    for (civ_entity, civilian, job) in civilians_with_jobs.iter_mut() {
        info!(
            "complete_improvement_jobs: checking {:?} {:?} job {:?} turns_remaining={}",
            civ_entity, civilian.kind, job.job_type, job.turns_remaining
        );

        // Only process jobs that just completed (turns_remaining == 0)
        if job.turns_remaining != 0 {
            continue;
        }

        info!(
            "complete_improvement_jobs: job completed for {:?}, processing {:?}",
            civ_entity, job.job_type
        );

        match job.job_type {
            JobType::ImprovingTile | JobType::Mining | JobType::Drilling => {
                // Find tile entity and complete improvement
                if let Some(tile_storage) = tile_storage_query.iter().next()
                    && let Some(tile_entity) = tile_storage.get(&job.target)
                    && let Ok(mut resource) = tile_resources.get_mut(tile_entity)
                {
                    if resource.improve() {
                        let action = match job.job_type {
                            JobType::Mining => "mining",
                            JobType::Drilling => "drilling",
                            _ => "improving",
                        };
                        info!(
                            "{:?} (owner: {:?}) completed {} {:?} at ({}, {}) to level {:?}",
                            civilian.kind,
                            civilian.owner,
                            action,
                            resource.resource_type,
                            job.target.x,
                            job.target.y,
                            resource.development
                        );

                        // Add visual improvement marker to the tile
                        commands.entity(tile_entity).insert(
                            crate::map::rendering::TileImprovement {
                                development_level: resource.development,
                            },
                        );
                    } else {
                        warn!(
                            "complete_improvement_jobs: resource.improve() returned false for {:?}",
                            resource.resource_type
                        );
                    }
                }
            }
            JobType::Prospecting => {
                if let Some(tile_storage) = tile_storage_query.iter().next()
                    && let Some(tile_entity) = tile_storage.get(&job.target)
                {
                    // Check if tile has potential mineral
                    if let Ok(potential) = potential_minerals.get(tile_entity) {
                        // Reveal what was found (or not found)
                        if let Some(resource_type) = potential.reveal() {
                            // Found a mineral! Create the TileResource
                            commands
                                .entity(tile_entity)
                                .insert(TileResource::visible(resource_type))
                                .insert(crate::map::ProspectedMineral { resource_type })
                                .remove::<crate::map::PotentialMineral>();

                            info!(
                                "Prospector (owner: {:?}) discovered {:?} at ({}, {})!",
                                civilian.owner, resource_type, job.target.x, job.target.y
                            );

                            // Mark as discovered for this nation
                            prospecting_knowledge.mark_discovered(tile_entity, civilian.owner);
                        } else {
                            // Nothing found
                            commands
                                .entity(tile_entity)
                                .insert(crate::map::ProspectedEmpty)
                                .remove::<crate::map::PotentialMineral>();

                            info!(
                                "Prospector (owner: {:?}) found no minerals at ({}, {})",
                                civilian.owner, job.target.x, job.target.y
                            );
                        }
                    }
                }
            }
            JobType::BuildingRail | JobType::BuildingDepot | JobType::BuildingPort => {
                // These are handled by the transport construction system
            }
        }

        // Remove the completed job and associated components
        commands
            .entity(civ_entity)
            .remove::<CivilianJob>()
            .remove::<PreviousPosition>()
            .remove::<ActionTurn>();
    }
}
