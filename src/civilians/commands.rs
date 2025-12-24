use bevy::prelude::*;

/// UI-only resource tracking which civilian is currently selected
/// This is purely for UI purposes and should not affect game logic
#[derive(Resource, Default, Debug)]
pub struct SelectedCivilian(pub Option<Entity>);

/// Message: Player selects a civilian unit
#[derive(Message, Debug, Clone, Copy)]
pub struct SelectCivilian {
    pub entity: Entity,
}

/// Message: Deselect the currently selected civilian
#[derive(Message, Debug)]
pub struct DeselectCivilian;

/// Message: Rescind orders for a civilian (undo their action this turn)
#[derive(Message, Debug, Clone, Copy)]
pub struct RescindOrders {
    pub entity: Entity,
}
