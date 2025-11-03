use bevy::prelude::*;

use crate::economy::NationInstance;
use crate::economy::workforce::WorkerSkill;

/// Message to queue recruitment of untrained workers at the Capitol.
#[derive(Message, Debug, Clone, Copy)]
pub struct RecruitWorkers {
    pub nation: NationInstance,
    pub count: u32,
}

/// Message to queue training of a worker at the Trade School.
#[derive(Message, Debug, Clone, Copy)]
pub struct TrainWorker {
    pub nation: NationInstance,
    pub from_skill: WorkerSkill,
}

#[cfg(test)]
mod tests {
    use crate::messages::*;
    use bevy::prelude::World;
    use moonshine_kind::Instance;

    use crate::economy::NationId;
    use crate::economy::workforce::WorkerSkill;
    use crate::messages::economy::AdjustTraining;

    #[test]
    fn workforce_messages_hold_expected_data() {
        let mut world = World::new();
        let nation_entity = world.spawn(NationId(42)).id();
        let nation = Instance::<NationId>::from_entity(world.entity(nation_entity))
            .expect("failed to build nation instance for test");

        let recruit = RecruitWorkers { nation, count: 12 };
        assert_eq!(recruit.count, 12);

        let train = TrainWorker {
            nation,
            from_skill: WorkerSkill::Untrained,
        };
        assert_eq!(train.from_skill, WorkerSkill::Untrained);

        // Ensure the module links correctly with other shared messages.
        fn assert_message_types<T: Send + Sync + 'static>() {}
        assert_message_types::<RecruitWorkers>();
        assert_message_types::<TrainWorker>();
        assert_message_types::<AdjustTraining>();
    }
}
