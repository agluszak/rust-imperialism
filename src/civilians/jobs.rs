use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::civilians::types::{
    ActionTurn, Civilian, CivilianJob, JobType, PreviousPosition, ProspectingKnowledge,
};
use crate::{resources::TileResource, ui::logging::TerminalLogEvent};

/// Reset civilian movement at start of player turn
pub fn reset_civilian_actions(mut civilians: Query<&mut Civilian>) {
    for mut civilian in civilians.iter_mut() {
        civilian.has_moved = false;
    }
}

/// Advance civilian jobs each turn
pub fn advance_civilian_jobs(
    mut commands: Commands,
    mut civilians_with_jobs: Query<(Entity, &mut CivilianJob)>,
) {
    for (entity, mut job) in civilians_with_jobs.iter_mut() {
        job.turns_remaining -= 1;

        if job.turns_remaining == 0 {
            info!("Job {:?} completed for civilian {:?}", job.job_type, entity);
            // Remove the job component and action tracking (job can no longer be rescinded)
            commands
                .entity(entity)
                .remove::<CivilianJob>()
                .remove::<PreviousPosition>()
                .remove::<ActionTurn>();
        } else {
            info!(
                "Job {:?} in progress for civilian {:?}: {} turns remaining",
                job.job_type, entity, job.turns_remaining
            );
        }
    }
}

/// Complete improvement jobs when they finish
pub fn complete_improvement_jobs(
    mut commands: Commands,
    civilians_with_jobs: Query<(&Civilian, &CivilianJob)>,
    tile_storage_query: Query<&TileStorage>,
    mut tile_resources: Query<&mut TileResource>,
    potential_minerals: Query<&crate::map::PotentialMineral>,
    mut prospecting_knowledge: ResMut<ProspectingKnowledge>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (civilian, job) in civilians_with_jobs.iter() {
        // Only process jobs that just completed (turns_remaining == 0)
        if job.turns_remaining != 0 {
            continue;
        }

        match job.job_type {
            JobType::ImprovingTile | JobType::Mining | JobType::Drilling => {
                // Find tile entity and complete improvement
                if let Some(tile_storage) = tile_storage_query.iter().next()
                    && let Some(tile_entity) = tile_storage.get(&job.target)
                    && let Ok(mut resource) = tile_resources.get_mut(tile_entity)
                    && resource.improve()
                {
                    let action = match job.job_type {
                        JobType::Mining => "mining",
                        JobType::Drilling => "drilling",
                        _ => "improving",
                    };
                    log_events.write(TerminalLogEvent {
                        message: format!(
                            "{:?} completed {} {:?} at ({}, {}) to level {:?}",
                            civilian.kind,
                            action,
                            resource.resource_type,
                            job.target.x,
                            job.target.y,
                            resource.development
                        ),
                    });

                    // Add visual improvement marker to the tile
                    commands
                        .entity(tile_entity)
                        .insert(crate::map::rendering::TileImprovement {
                            development_level: resource.development,
                        });
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

                            log_events.write(TerminalLogEvent {
                                message: format!(
                                    "Prospector discovered {:?} at ({}, {})!",
                                    resource_type, job.target.x, job.target.y
                                ),
                            });

                            // Mark as discovered for this nation
                            prospecting_knowledge.mark_discovered(tile_entity, civilian.owner);
                        } else {
                            // Nothing found
                            commands
                                .entity(tile_entity)
                                .insert(crate::map::ProspectedEmpty)
                                .remove::<crate::map::PotentialMineral>();

                            log_events.write(TerminalLogEvent {
                                message: format!(
                                    "Prospector found no minerals at ({}, {})",
                                    job.target.x, job.target.y
                                ),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
