use bevy::prelude::*;

use crate::economy::NationId;

/// Orders issued during the player turn or by future AI actors.
#[derive(Message, Debug, Clone)]
pub struct DiplomaticOrder {
    pub actor: NationId,
    pub target: NationId,
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
    use super::*;

    #[test]
    fn diplomatic_orders_are_send_sync() {
        fn assert_message<T: Send + Sync + 'static>() {}

        assert_message::<DiplomaticOrder>();
    }
}
