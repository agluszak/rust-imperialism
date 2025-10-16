use bevy::prelude::*;

use crate::economy::production::BuildingKind;

/// Marker for a building dialog window
#[derive(Component, Clone)]
pub struct BuildingDialog {
    pub building_entity: Entity,
    pub building_kind: BuildingKind,
    pub z_index: i32,           // For window stacking
    pub content_entity: Entity, // The DialogContentArea entity for this dialog
}

/// Message to open a building dialog
#[derive(Message, Debug, Clone, Copy)]
pub struct OpenBuildingDialog {
    pub building_entity: Entity,
    pub building_kind: BuildingKind,
}

/// Message to close a building dialog
#[derive(Message, Debug, Clone, Copy)]
pub struct CloseBuildingDialog {
    pub building_entity: Entity,
}

/// Marker for the close button in a dialog
#[derive(Component)]
pub struct DialogCloseButton {
    pub building_entity: Entity,
}

/// Resource tracking the next z-index for dialogs
#[derive(Resource, Default)]
pub struct DialogZIndexCounter {
    pub next: i32,
}

impl DialogZIndexCounter {
    pub fn get_next(&mut self) -> i32 {
        let current = self.next;
        self.next += 1;
        current
    }
}

/// Marker for the content area inside a dialog
#[derive(Component)]
pub struct DialogContentArea;

/// Marker for the draggable header area of a dialog
#[derive(Component)]
pub struct DialogDragHandle {
    pub dialog_entity: Entity,
}

/// State for tracking dialog dragging
#[derive(Component)]
pub struct DialogDragState {
    pub is_dragging: bool,
    pub drag_offset: Vec2, // Offset from top-left corner to mouse position when drag started
}
