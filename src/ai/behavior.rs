use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};
use big_brain::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::ai::context::enemy_turn_entered;
use crate::ai::markers::{AiControlledCivilian, AiNation};
use crate::civilians::order_validation::tile_owned_by_nation;
use crate::civilians::systems::handle_civilian_commands;
use crate::civilians::types::{Civilian, CivilianOrder, CivilianOrderKind};
use crate::map::province::{Province, TileProvince};
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::messages::civilians::CivilianCommand;
use crate::turn_system::{TurnPhase, TurnSystem};
use crate::ui::menu::AppState;

pub(crate) const RNG_BASE_SEED: u64 = 0xA1_51_23_45;

/// Registers Big Brain and the systems that drive simple AI-controlled civilians.
pub struct AiBehaviorPlugin;

impl Plugin for AiBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiRng>()
            .add_plugins(BigBrainPlugin::new(Update))
            .add_systems(
                Update,
                (
                    tag_ai_owned_civilians,
                    untag_non_ai_owned_civilians,
                    initialize_ai_thinkers,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                reset_ai_rng_on_enemy_turn
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_entered),
            )
            .add_systems(
                Update,
                ready_to_act_scorer
                    .in_set(BigBrainSet::Scorers)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active),
            )
            .add_systems(
                Update,
                issue_ai_orders_action
                    .in_set(BigBrainSet::Actions)
                    .run_if(in_state(AppState::InGame))
                    .run_if(enemy_turn_active)
                    .before(handle_civilian_commands),
            );
    }
}

/// Simple deterministic RNG used by the AI so behavior is replayable.
#[derive(Resource)]
pub struct AiRng(StdRng);

impl Default for AiRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(RNG_BASE_SEED))
    }
}

fn tag_ai_owned_civilians(
    mut commands: Commands,
    ai_nations: Query<(), With<AiNation>>,
    untagged: Query<(Entity, &Civilian), Without<AiControlledCivilian>>,
) {
    for (entity, civilian) in &untagged {
        if ai_nations.get(civilian.owner).is_ok() {
            commands.entity(entity).insert(AiControlledCivilian);
        }
    }
}

fn untag_non_ai_owned_civilians(
    mut commands: Commands,
    ai_nations: Query<(), With<AiNation>>,
    tagged: Query<(Entity, &Civilian), With<AiControlledCivilian>>,
) {
    for (entity, civilian) in &tagged {
        if ai_nations.get(civilian.owner).is_err() {
            commands.entity(entity).remove::<AiControlledCivilian>();
        }
    }
}

/// Scorer that determines whether a civilian is ready to issue a new order.
#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct ReadyToAct;

/// Action that picks and sends an order for an AI-controlled civilian.
#[derive(Component, Debug, Clone, Default, ActionBuilder)]
pub struct IssueAiOrder;

fn enemy_turn_active(turn: Res<TurnSystem>) -> bool {
    turn.phase == TurnPhase::EnemyTurn
}

fn reset_ai_rng_on_enemy_turn(mut rng: ResMut<AiRng>, turn: Res<TurnSystem>) {
    rng.0 = StdRng::seed_from_u64(RNG_BASE_SEED ^ u64::from(turn.current_turn));
}

fn initialize_ai_thinkers(
    mut commands: Commands,
    new_units: Query<Entity, (With<AiControlledCivilian>, Without<Thinker>)>,
) {
    for entity in &new_units {
        commands.entity(entity).insert(
            Thinker::build()
                .label("ai_civilian")
                .picker(FirstToScore { threshold: 0.5 })
                .when(ReadyToAct, IssueAiOrder::default()),
        );
    }
}

fn ready_to_act_scorer(
    civilians: Query<&Civilian>,
    mut scores: Query<(&Actor, &mut Score, &ScorerSpan), With<ReadyToAct>>,
) {
    for (Actor(actor), mut score, span) in &mut scores {
        let readiness = civilians
            .get(*actor)
            .map(|civilian| if civilian.has_moved { 0.0 } else { 0.8 })
            .unwrap_or_default();

        span.span().in_scope(|| {
            trace!("AI scorer for {:?}: {}", actor, readiness);
        });

        score.set(readiness);
    }
}

fn issue_ai_orders_action(
    mut command_writer: MessageWriter<CivilianCommand>,
    mut rng: ResMut<AiRng>,
    civilians: Query<(&Civilian, Option<&CivilianOrder>)>,
    tile_storage_query: Query<&TileStorage>,
    tile_provinces: Query<&TileProvince>,
    provinces: Query<&Province>,
    mut actions: Query<(&Actor, &mut ActionState, &IssueAiOrder, &ActionSpan)>,
) {
    let tile_storage = tile_storage_query.iter().next();

    for (Actor(actor), mut state, _action, span) in &mut actions {
        let _guard = span.span().enter();

        match *state {
            ActionState::Init => {
                *state = ActionState::Requested;
            }
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            ActionState::Executing => {
                let Ok((civilian, pending_order)) = civilians.get(*actor) else {
                    *state = ActionState::Failure;
                    continue;
                };

                if civilian.has_moved || pending_order.is_some() {
                    *state = ActionState::Success;
                    continue;
                }

                let order = tile_storage
                    .and_then(|storage| {
                        select_move_target(
                            civilian,
                            storage,
                            &tile_provinces,
                            &provinces,
                            &mut rng.0,
                        )
                    })
                    .unwrap_or(CivilianOrderKind::SkipTurn);

                command_writer.write(CivilianCommand {
                    civilian: *actor,
                    order,
                });

                *state = ActionState::Success;
            }
            ActionState::Success | ActionState::Failure => {}
        }
    }
}

fn select_move_target(
    civilian: &Civilian,
    storage: &TileStorage,
    tile_provinces: &Query<&TileProvince>,
    provinces: &Query<&Province>,
    rng: &mut StdRng,
) -> Option<CivilianOrderKind> {
    let mut candidates: Vec<TilePos> = civilian
        .position
        .to_hex()
        .all_neighbors()
        .iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| {
            tile_owned_by_nation(*pos, civilian.owner, storage, tile_provinces, provinces)
        })
        .collect();

    candidates.shuffle(rng);
    let target = candidates.first().copied()?;

    Some(CivilianOrderKind::Move { to: target })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{Commands, Entity, Query, Res, ResMut, With, Without, World};
    use bevy_ecs_tilemap::prelude::{TilePos, TileStorage, TilemapSize};
    use rand::RngCore;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::ai::behavior::{
        AiRng, RNG_BASE_SEED, reset_ai_rng_on_enemy_turn, select_move_target,
        tag_ai_owned_civilians, untag_non_ai_owned_civilians,
    };
    use crate::ai::markers::{AiControlledCivilian, AiNation};
    use crate::civilians::types::{Civilian, CivilianKind, CivilianOrderKind};
    use crate::map::province::{Province, ProvinceId, TileProvince};
    use crate::turn_system::{TurnPhase, TurnSystem};

    #[test]
    fn tags_ai_owned_civilians() {
        let mut world = World::new();
        let ai_nation = world.spawn(AiNation).id();
        let civilian_entity = world
            .spawn(Civilian {
                kind: CivilianKind::Engineer,
                position: TilePos { x: 1, y: 1 },
                owner: ai_nation,
                selected: false,
                has_moved: false,
            })
            .id();

        let mut state: SystemState<(
            Commands,
            Query<(), With<AiNation>>,
            Query<(Entity, &Civilian), Without<AiControlledCivilian>>,
        )> = SystemState::new(&mut world);

        {
            let (commands, ai_nations, untagged) = state.get_mut(&mut world);
            tag_ai_owned_civilians(commands, ai_nations, untagged);
        }
        state.apply(&mut world);

        assert!(world.get::<AiControlledCivilian>(civilian_entity).is_some());
    }

    #[test]
    fn removes_tag_from_non_ai_owned_civilians() {
        let mut world = World::new();
        let player_nation = world.spawn_empty().id();
        let civilian_entity = world
            .spawn((
                Civilian {
                    kind: CivilianKind::Engineer,
                    position: TilePos { x: 1, y: 1 },
                    owner: player_nation,
                    selected: false,
                    has_moved: false,
                },
                AiControlledCivilian,
            ))
            .id();

        let mut state: SystemState<(
            Commands,
            Query<(), With<AiNation>>,
            Query<(Entity, &Civilian), With<AiControlledCivilian>>,
        )> = SystemState::new(&mut world);

        {
            let (commands, ai_nations, tagged) = state.get_mut(&mut world);
            untag_non_ai_owned_civilians(commands, ai_nations, tagged);
        }
        state.apply(&mut world);

        assert!(world.get::<AiControlledCivilian>(civilian_entity).is_none());
    }

    #[test]
    fn reseeds_rng_when_enemy_turn_begins() {
        let mut world = World::new();
        world.insert_resource(AiRng(StdRng::seed_from_u64(0xDEADBEEF)));
        world.insert_resource(TurnSystem {
            current_turn: 7,
            phase: TurnPhase::EnemyTurn,
        });

        let mut state: SystemState<(ResMut<AiRng>, Res<TurnSystem>)> = SystemState::new(&mut world);

        {
            let (rng, turn) = state.get_mut(&mut world);
            reset_ai_rng_on_enemy_turn(rng, turn);
        }
        state.apply(&mut world);

        let mut expected = StdRng::seed_from_u64(RNG_BASE_SEED ^ 7);
        let mut rng = world.resource_mut::<AiRng>();
        let actual = rng.0.next_u32();
        let expected_value = expected.next_u32();
        assert_eq!(actual, expected_value);
    }

    #[test]
    fn selects_owned_neighbor_as_move_target() {
        let mut world = World::new();
        let ai_nation = world.spawn(AiNation).id();
        let neighbor_pos = TilePos { x: 2, y: 1 };
        let mut storage = TileStorage::empty(TilemapSize { x: 4, y: 4 });

        let province_id = ProvinceId(1);
        let start_tile = world.spawn(TileProvince { province_id }).id();
        let neighbor_tile = world.spawn(TileProvince { province_id }).id();

        storage.set(&TilePos { x: 1, y: 1 }, start_tile);
        storage.set(&neighbor_pos, neighbor_tile);

        world.spawn(Province {
            id: province_id,
            tiles: vec![TilePos { x: 1, y: 1 }, neighbor_pos],
            city_tile: TilePos { x: 1, y: 1 },
            owner: Some(ai_nation),
        });

        let civilian = Civilian {
            kind: CivilianKind::Engineer,
            position: TilePos { x: 1, y: 1 },
            owner: ai_nation,
            selected: false,
            has_moved: false,
        };

        let mut state: SystemState<(Query<&TileProvince>, Query<&Province>)> =
            SystemState::new(&mut world);

        let mut rng = StdRng::seed_from_u64(42);
        let order = {
            let (tile_provinces, provinces) = state.get(&mut world);
            let order =
                select_move_target(&civilian, &storage, &tile_provinces, &provinces, &mut rng);
            order
        };
        state.apply(&mut world);

        match order {
            Some(CivilianOrderKind::Move { to }) => assert_eq!(to, neighbor_pos),
            _ => panic!("expected move order"),
        }
    }
}
