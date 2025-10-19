use bevy::prelude::*;

use crate::economy::workforce::WorkerSkill;
use crate::economy::{NationInstance, goods::Good};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketInterest {
    Buy,
    Sell,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustRecruitment {
    pub nation: NationInstance,
    pub requested: u32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustTraining {
    pub nation: NationInstance,
    pub from_skill: WorkerSkill,
    pub requested: u32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustProduction {
    pub nation: NationInstance,
    pub building: Entity,
    pub output_good: Good,
    pub target_output: u32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct AdjustMarketOrder {
    pub nation: NationInstance,
    pub good: Good,
    pub kind: MarketInterest,
    pub requested: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;
    use moonshine_kind::Instance;

    use crate::economy::NationId;
    use crate::economy::goods::Good;
    use crate::economy::workforce::WorkerSkill;

    #[test]
    fn adjusts_hold_data() {
        let mut world = World::new();
        let nation_entity = world.spawn(NationId(1)).id();
        let nation = Instance::<NationId>::from_entity(world.entity(nation_entity))
            .expect("failed to build nation instance for test");
        let building = world.spawn_empty().id();
        let recruit = AdjustRecruitment {
            nation,
            requested: 5,
        };
        assert_eq!(recruit.requested, 5);

        let training = AdjustTraining {
            nation,
            from_skill: WorkerSkill::Untrained,
            requested: 3,
        };
        assert_eq!(training.from_skill, WorkerSkill::Untrained);

        let production = AdjustProduction {
            nation,
            building,
            output_good: Good::Fabric,
            target_output: 4,
        };
        assert_eq!(production.target_output, 4);

        let market = AdjustMarketOrder {
            nation,
            good: Good::Cotton,
            kind: MarketInterest::Buy,
            requested: 7,
        };
        assert_eq!(market.kind, MarketInterest::Buy);
    }
}
