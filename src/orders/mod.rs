use bevy::prelude::*;

use crate::messages::{AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining};

/// Queue of structured orders emitted during a nation's turn.
///
/// Orders are accumulated while the player (or AI) issues commands and are
/// executed in a dedicated phase before Processing begins.
#[derive(Resource, Default, Debug)]
pub struct OrdersQueue {
    production: Vec<AdjustProduction>,
    recruitment: Vec<AdjustRecruitment>,
    training: Vec<AdjustTraining>,
    market: Vec<AdjustMarketOrder>,
}

impl OrdersQueue {
    pub fn queue_production(&mut self, order: AdjustProduction) {
        self.production.push(order);
    }

    pub fn queue_recruitment(&mut self, order: AdjustRecruitment) {
        self.recruitment.push(order);
    }

    pub fn queue_training(&mut self, order: AdjustTraining) {
        self.training.push(order);
    }

    pub fn queue_market(&mut self, order: AdjustMarketOrder) {
        self.market.push(order);
    }

    pub fn take_production(&mut self) -> Vec<AdjustProduction> {
        std::mem::take(&mut self.production)
    }

    pub fn take_recruitment(&mut self) -> Vec<AdjustRecruitment> {
        std::mem::take(&mut self.recruitment)
    }

    pub fn take_training(&mut self) -> Vec<AdjustTraining> {
        std::mem::take(&mut self.training)
    }

    pub fn take_market(&mut self) -> Vec<AdjustMarketOrder> {
        std::mem::take(&mut self.market)
    }

    pub fn is_empty(&self) -> bool {
        self.production.is_empty()
            && self.recruitment.is_empty()
            && self.training.is_empty()
            && self.market.is_empty()
    }

    pub fn clear(&mut self) {
        self.production.clear();
        self.recruitment.clear();
        self.training.clear();
        self.market.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::orders::*;
    use bevy::prelude::World;
    use moonshine_kind::Instance;

    use crate::economy::workforce::WorkerSkill;
    use crate::economy::{NationId, goods::Good};

    #[test]
    fn queue_and_take_orders() {
        let mut world = World::new();
        let nation_entity = world.spawn(NationId(7)).id();
        let nation = Instance::<NationId>::from_entity(world.entity(nation_entity))
            .expect("failed to build nation instance for test");
        let building = world.spawn_empty().id();

        let mut queue = OrdersQueue::default();
        queue.queue_production(AdjustProduction {
            nation,
            building,
            output_good: Good::Steel,
            target_output: 3,
        });
        queue.queue_recruitment(AdjustRecruitment {
            nation,
            requested: 2,
        });
        queue.queue_training(AdjustTraining {
            nation,
            from_skill: WorkerSkill::Untrained,
            requested: 1,
        });
        queue.queue_market(AdjustMarketOrder {
            nation,
            good: Good::Cotton,
            kind: crate::messages::MarketInterest::Buy,
            requested: 5,
        });

        assert!(!queue.is_empty());

        assert_eq!(queue.take_production().len(), 1);
        assert_eq!(queue.take_recruitment().len(), 1);
        assert_eq!(queue.take_training().len(), 1);
        assert_eq!(queue.take_market().len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn clear_discards_orders() {
        let mut queue = OrdersQueue::default();
        let mut world = World::new();
        let nation_entity = world.spawn(NationId(1)).id();
        let nation = Instance::<NationId>::from_entity(world.entity(nation_entity))
            .expect("failed to build nation instance for test");

        queue.queue_recruitment(AdjustRecruitment {
            nation,
            requested: 4,
        });

        queue.clear();
        assert!(queue.is_empty());
    }
}
