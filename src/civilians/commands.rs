use bevy::prelude::*;

use super::types::CivilianOrderKind;

/// Message: Player selects a civilian unit
#[derive(Message, Debug, Clone, Copy)]
pub struct SelectCivilian {
    pub entity: Entity,
}

/// Message: Player gives an order to selected civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct GiveCivilianOrder {
    pub entity: Entity,
    pub order: CivilianOrderKind,
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
