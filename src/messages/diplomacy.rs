use bevy::prelude::*;

use crate::economy::NationInstance;

/// Orders issued during the player turn or by future AI actors.
#[derive(Event, Debug, Clone)]
pub struct DiplomaticOrder {
    pub actor: NationInstance,
    pub target: NationInstance,
    pub kind: DiplomaticOrderKind,
}

#[derive(Debug, Clone)]
pub enum DiplomaticOrderKind {
    DeclareWar,
    OfferPeace,
    EstablishConsulate,
    OpenEmbassy,
    SignNonAggressionPact,
    FormAlliance,
    SendAid { amount: i32, locked: bool },
    CancelAid,
}

#[cfg(test)]
mod tests {
    use crate::messages::*;

    #[test]
    fn diplomatic_orders_are_send_sync() {
        fn assert_message<T: Send + Sync + 'static>() {}

        assert_message::<DiplomaticOrder>();
    }
}
