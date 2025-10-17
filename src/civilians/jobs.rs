use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use super::types::{ActionTurn, Civilian, CivilianJob, JobType, PreviousPosition};
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
    civilians_with_jobs: Query<(&Civilian, &CivilianJob)>,
    tile_storage_query: Query<&TileStorage>,
    mut tile_resources: Query<&mut TileResource>,
    mut log_events: MessageWriter<TerminalLogEvent>,
) {
    for (civilian, job) in civilians_with_jobs.iter() {
        // Only process jobs that just completed (turns_remaining == 0)
        if job.turns_remaining != 0 {
            continue;
        }

        // Only process improvement jobs
        if job.job_type != JobType::ImprovingTile {
            continue;
        }

        // Find tile entity and complete improvement
        if let Some(tile_storage) = tile_storage_query.iter().next()
            && let Some(tile_entity) = tile_storage.get(&job.target)
            && let Ok(mut resource) = tile_resources.get_mut(tile_entity)
            && resource.improve()
        {
            log_events.write(TerminalLogEvent {
                message: format!(
                    "{:?} completed improving {:?} at ({}, {}) to level {:?}",
                    civilian.kind,
                    resource.resource_type,
                    job.target.x,
                    job.target.y,
                    resource.development
                ),
            });
        }
    }
}
