use bevy::prelude::*;

/// Message: Player selects a civilian unit
#[derive(Message, Debug, Clone, Copy)]
pub struct SelectCivilian {
    pub entity: Entity,
}

/// Message: Deselect a specific civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct DeselectCivilian {
    pub entity: Entity,
}

/// Message: Deselect all civilians
#[derive(Message, Debug)]
pub struct DeselectAllCivilians;

/// Message: Rescind orders for a civilian (undo their action this turn)
#[derive(Message, Debug, Clone, Copy)]
pub struct RescindOrders {
    pub entity: Entity,
}
