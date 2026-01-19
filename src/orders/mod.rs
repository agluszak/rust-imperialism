use bevy::prelude::*;

use crate::economy::transport::PlaceImprovement;
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
    transport: Vec<PlaceImprovement>,
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

    pub fn queue_transport(&mut self, order: PlaceImprovement) {
        self.transport.push(order);
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

    pub fn take_transport(&mut self) -> Vec<PlaceImprovement> {
        std::mem::take(&mut self.transport)
    }

    pub fn peek_production(&self) -> &[AdjustProduction] {
        &self.production
    }

    pub fn peek_market(&self) -> &[AdjustMarketOrder] {
        &self.market
    }

    pub fn is_empty(&self) -> bool {
        self.production.is_empty()
            && self.recruitment.is_empty()
            && self.training.is_empty()
            && self.market.is_empty()
            && self.transport.is_empty()
    }

    pub fn clear(&mut self) {
        self.production.clear();
        self.recruitment.clear();
        self.training.clear();
        self.market.clear();
        self.transport.clear();
    }
}

/// Buffer for AI-generated orders that need to be merged into the global queue.
#[derive(Resource, Default, Debug)]
pub struct OrdersOut {
    production: Vec<AdjustProduction>,
    recruitment: Vec<AdjustRecruitment>,
    training: Vec<AdjustTraining>,
    market: Vec<AdjustMarketOrder>,
    transport: Vec<PlaceImprovement>,
}

impl OrdersOut {
    pub fn queue_market(&mut self, order: AdjustMarketOrder) {
        self.market.push(order);
    }

    pub fn queue_transport(&mut self, order: PlaceImprovement) {
        self.transport.push(order);
    }

    pub fn queue_production(&mut self, order: AdjustProduction) {
        self.production.push(order);
    }

    pub fn queue_recruitment(&mut self, order: AdjustRecruitment) {
        self.recruitment.push(order);
    }

    pub fn queue_training(&mut self, order: AdjustTraining) {
        self.training.push(order);
    }

    pub fn clear(&mut self) {
        self.production.clear();
        self.recruitment.clear();
        self.training.clear();
        self.market.clear();
        self.transport.clear();
    }
}

/// Moves buffered AI orders into the shared queue so they can be executed alongside
/// player-issued commands.
pub fn flush_orders_to_queue(mut src: ResMut<OrdersOut>, mut dst: ResMut<OrdersQueue>) {
    for order in src.production.drain(..) {
        dst.queue_production(order);
    }
    for order in src.recruitment.drain(..) {
        dst.queue_recruitment(order);
    }
    for order in src.training.drain(..) {
        dst.queue_training(order);
    }
    for order in src.market.drain(..) {
        dst.queue_market(order);
    }
    for order in src.transport.drain(..) {
        dst.queue_transport(order);
    }
}

#[cfg(test)]
mod tests {
    use crate::orders::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{ResMut, World};
    use moonshine_kind::Instance;

    use crate::economy::transport::{ImprovementKind, PlaceImprovement};
    use crate::economy::workforce::WorkerSkill;
    use crate::economy::{Nation, goods::Good};

    #[test]
    fn queue_and_take_orders() {
        let mut world = World::new();
        let nation_entity = world.spawn(Nation).id();
        let nation = Instance::<Nation>::from_entity(world.entity(nation_entity))
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
        let improvement = PlaceImprovement {
            a: bevy_ecs_tilemap::prelude::TilePos { x: 1, y: 2 },
            b: bevy_ecs_tilemap::prelude::TilePos { x: 1, y: 3 },
            kind: ImprovementKind::Rail,
            nation: None,
            engineer: None,
        };

        queue.queue_market(AdjustMarketOrder {
            nation,
            good: Good::Cotton,
            kind: crate::messages::MarketInterest::Buy,
            requested: 5,
        });
        queue.queue_transport(improvement);

        assert!(!queue.is_empty());

        assert_eq!(queue.take_production().len(), 1);
        assert_eq!(queue.take_recruitment().len(), 1);
        assert_eq!(queue.take_training().len(), 1);
        assert_eq!(queue.take_market().len(), 1);
        assert_eq!(queue.take_transport().len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn clear_discards_orders() {
        let mut queue = OrdersQueue::default();
        let mut world = World::new();
        let nation_entity = world.spawn(Nation).id();
        let nation = Instance::<Nation>::from_entity(world.entity(nation_entity))
            .expect("failed to build nation instance for test");

        queue.queue_recruitment(AdjustRecruitment {
            nation,
            requested: 4,
        });
        queue.queue_transport(PlaceImprovement {
            a: bevy_ecs_tilemap::prelude::TilePos { x: 0, y: 0 },
            b: bevy_ecs_tilemap::prelude::TilePos { x: 1, y: 0 },
            kind: ImprovementKind::Rail,
            nation: None,
            engineer: None,
        });

        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn flushes_buffered_orders() {
        let mut world = World::new();
        world.insert_resource(OrdersQueue::default());
        world.insert_resource(OrdersOut::default());

        let nation_entity = world.spawn(Nation).id();
        let nation = Instance::<Nation>::from_entity(world.entity(nation_entity)).unwrap();
        {
            let mut world_queue = world.resource_mut::<OrdersOut>();
            world_queue.queue_market(AdjustMarketOrder {
                nation,
                good: Good::Coal,
                kind: crate::messages::MarketInterest::Buy,
                requested: 2,
            });
            world_queue.queue_transport(PlaceImprovement {
                a: bevy_ecs_tilemap::prelude::TilePos { x: 0, y: 0 },
                b: bevy_ecs_tilemap::prelude::TilePos { x: 0, y: 1 },
                kind: ImprovementKind::Rail,
                nation: None,
                engineer: None,
            });
        }

        let _ = world.run_system_once(|out: ResMut<OrdersOut>, queue: ResMut<OrdersQueue>| {
            flush_orders_to_queue(out, queue);
        });

        let mut queue = world.resource_mut::<OrdersQueue>();
        assert_eq!(queue.take_market().len(), 1);
        assert_eq!(queue.take_transport().len(), 1);
    }
}
