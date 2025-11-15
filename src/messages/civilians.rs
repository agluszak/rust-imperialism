use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::civilians::{CivilianKind, CivilianOrderKind};
use crate::economy::nation::NationInstance;

#[derive(Message, Debug, Clone, Copy)]
pub struct CivilianCommand {
    pub civilian: Entity,
    pub order: CivilianOrderKind,
}

/// Message sent when a nation hires a new civilian unit.
#[derive(Message, Debug, Clone, Copy)]
pub struct HireCivilian {
    pub nation: NationInstance,
    pub kind: CivilianKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilianCommandError {
    MissingCivilian,
    AlreadyHasJob,
    AlreadyActed,
    CurrentTileUnowned,
    TargetTileUnowned,
    RequiresEngineer,
    RequiresProspector,
    RequiresImprover,
    MissingTileStorage,
    MissingTargetTile(TilePos),
}

impl CivilianCommandError {
    pub fn describe(self) -> &'static str {
        match self {
            CivilianCommandError::MissingCivilian => "civilian not found",
            CivilianCommandError::AlreadyHasJob => "civilian already has an active job",
            CivilianCommandError::AlreadyActed => "civilian has already acted this turn",
            CivilianCommandError::CurrentTileUnowned => {
                "current tile is not owned by issuing nation"
            }
            CivilianCommandError::TargetTileUnowned => "target tile is not owned by issuing nation",
            CivilianCommandError::RequiresEngineer => "order requires an engineer",
            CivilianCommandError::RequiresProspector => "order requires a prospector",
            CivilianCommandError::RequiresImprover => "order requires a resource improver",
            CivilianCommandError::MissingTileStorage => "no tile storage available",
            CivilianCommandError::MissingTargetTile(_) => "target tile does not exist",
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct CivilianCommandRejected {
    pub civilian: Entity,
    pub order: CivilianOrderKind,
    pub reason: CivilianCommandError,
}

#[cfg(test)]
mod tests {
    use crate::messages::*;

    #[test]
    fn command_error_descriptions_are_static() {
        assert_eq!(
            CivilianCommandError::MissingCivilian.describe(),
            "civilian not found"
        );
        assert_eq!(
            CivilianCommandError::RequiresEngineer.describe(),
            "order requires an engineer"
        );
    }
}
