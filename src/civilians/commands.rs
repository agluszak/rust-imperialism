use bevy::prelude::*;

/// UI-only resource tracking which civilian is currently selected.
/// This resource exists only while a civilian is selected.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SelectedCivilian(pub Entity);

/// Message: Player selects a civilian unit
#[derive(Event, Debug, Clone, Copy)]
pub struct SelectCivilian {
    pub entity: Entity,
}

/// Message: Deselect the currently selected civilian
#[derive(Event, Debug)]
pub struct DeselectCivilian;

/// Message: Rescind orders for a civilian (undo their action this turn)
#[derive(Event, Debug, Clone, Copy)]
pub struct RescindOrders {
    pub entity: Entity,
}
