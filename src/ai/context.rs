use bevy::prelude::*;
use std::collections::BTreeSet;

use crate::economy::allocation::Allocations;
use crate::economy::goods::Good;
use crate::economy::nation::NationId;
use crate::economy::stockpile::{Stockpile, StockpileEntry};
use crate::economy::transport::{
    CapacitySnapshot, DemandEntry, TransportAllocations, TransportCapacity, TransportCommodity,
    TransportDemandSnapshot,
};
use crate::economy::workforce::{WorkerSkill, Workforce};
use crate::turn_system::{TurnPhase, TurnSystem};

pub type AiStockpileEntry = StockpileEntry;

#[derive(Resource, Debug, Clone)]
pub struct AiTurnContext {
    turn: u32,
    phase: TurnPhase,
    nations: Vec<AiNationSnapshot>,
}

impl Default for AiTurnContext {
    fn default() -> Self {
        Self {
            turn: 0,
            phase: TurnPhase::PlayerTurn,
            nations: Vec::new(),
        }
    }
}

impl AiTurnContext {
    pub fn clear(&mut self) {
        self.turn = 0;
        self.phase = TurnPhase::PlayerTurn;
        self.nations.clear();
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }

    pub fn phase(&self) -> TurnPhase {
        self.phase
    }

    pub fn is_empty(&self) -> bool {
        self.nations.is_empty()
    }

    pub fn nations(&self) -> &[AiNationSnapshot] {
        &self.nations
    }

    pub fn for_nation(&self, nation: Entity) -> Option<&AiNationSnapshot> {
        self.nations
            .iter()
            .find(|snapshot| snapshot.entity == nation)
    }
}

#[derive(Debug, Clone)]
pub struct AiNationSnapshot {
    pub entity: Entity,
    pub id: NationId,
    pub stockpile: Vec<AiStockpileEntry>,
    pub workforce: AiWorkforceSnapshot,
    pub allocations: AiAllocationSnapshot,
    pub transport: AiTransportSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct AiWorkforceSnapshot {
    pub untrained: u32,
    pub trained: u32,
    pub expert: u32,
    pub available_labor: u32,
}

impl AiWorkforceSnapshot {
    fn from_workforce(workforce: &Workforce) -> Self {
        Self {
            untrained: workforce.untrained_count(),
            trained: workforce.trained_count(),
            expert: workforce.expert_count(),
            available_labor: workforce.available_labor(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AiAllocationSnapshot {
    pub production: Vec<AiProductionAllocation>,
    pub recruitment: usize,
    pub training: Vec<AiTrainingAllocation>,
    pub market_buy_interest: Vec<Good>,
    pub market_sells: Vec<AiMarketSell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiProductionAllocation {
    pub building: Entity,
    pub output: Good,
    pub reserved: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiTrainingAllocation {
    pub skill: WorkerSkill,
    pub reserved: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiMarketSell {
    pub good: Good,
    pub reserved: usize,
}

impl AiAllocationSnapshot {
    fn from_allocations(allocations: Option<&Allocations>) -> Self {
        let mut snapshot = Self::default();

        if let Some(allocations) = allocations {
            snapshot.recruitment = allocations.recruitment.len();

            for ((building, good), reservations) in allocations.production.iter() {
                snapshot.production.push(AiProductionAllocation {
                    building: *building,
                    output: *good,
                    reserved: reservations.len(),
                });
            }
            snapshot
                .production
                .sort_by_key(|entry| (entry.building.index(), entry.output));

            for (skill, reservations) in allocations.training.iter() {
                snapshot.training.push(AiTrainingAllocation {
                    skill: *skill,
                    reserved: reservations.len(),
                });
            }
            snapshot
                .training
                .sort_by_key(|entry| training_skill_order(entry.skill));

            let buy_interest: BTreeSet<Good> =
                allocations.market_buy_interest.iter().copied().collect();
            snapshot.market_buy_interest = buy_interest.into_iter().collect();

            for (good, reservations) in allocations.market_sells.iter() {
                snapshot.market_sells.push(AiMarketSell {
                    good: *good,
                    reserved: reservations.len(),
                });
            }
            snapshot.market_sells.sort_by_key(|entry| entry.good);
        }

        snapshot
    }
}

fn training_skill_order(skill: WorkerSkill) -> u8 {
    match skill {
        WorkerSkill::Untrained => 0,
        WorkerSkill::Trained => 1,
        WorkerSkill::Expert => 2,
    }
}

#[derive(Debug, Clone, Default)]
pub struct AiTransportSnapshot {
    pub capacity: CapacitySnapshot,
    pub allocations: Vec<AiTransportAllocation>,
    pub demand: Vec<AiTransportDemand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiTransportAllocation {
    pub commodity: TransportCommodity,
    pub requested: u32,
    pub granted: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiTransportDemand {
    pub commodity: TransportCommodity,
    pub supply: u32,
    pub demand: u32,
}

impl AiTransportSnapshot {
    fn from_resources(
        nation: Entity,
        capacity: Option<&TransportCapacity>,
        allocations: Option<&TransportAllocations>,
        demand: Option<&TransportDemandSnapshot>,
    ) -> Self {
        let mut snapshot = Self::default();

        if let Some(capacity) = capacity {
            snapshot.capacity = capacity.snapshot(nation);
        }

        if let Some(allocations) = allocations {
            for commodity in TransportCommodity::ORDERED {
                let slot = allocations.slot(nation, commodity);
                if slot.requested > 0 || slot.granted > 0 {
                    snapshot.allocations.push(AiTransportAllocation {
                        commodity,
                        requested: slot.requested,
                        granted: slot.granted,
                    });
                }
            }
        }

        if let Some(demand) = demand
            && let Some(entries) = demand.nations.get(&nation)
        {
            for commodity in TransportCommodity::ORDERED {
                if let Some(DemandEntry { supply, demand }) = entries.get(&commodity)
                    && (*supply > 0 || *demand > 0)
                {
                    snapshot.demand.push(AiTransportDemand {
                        commodity,
                        supply: *supply,
                        demand: *demand,
                    });
                }
            }
        }

        snapshot
    }
}

/// Run condition that returns true when the enemy turn has just begun.
pub fn enemy_turn_entered(mut last_phase: Local<Option<TurnPhase>>, turn: Res<TurnSystem>) -> bool {
    let previous = *last_phase;
    let current = turn.phase;
    *last_phase = Some(current);
    current == TurnPhase::EnemyTurn && previous != Some(TurnPhase::EnemyTurn)
}

pub fn populate_ai_turn_context(
    mut context: ResMut<AiTurnContext>,
    turn: Res<TurnSystem>,
    nations: Query<(
        Entity,
        &NationId,
        &Stockpile,
        &Workforce,
        Option<&Allocations>,
    )>,
    transport_capacity: Option<Res<TransportCapacity>>,
    transport_allocations: Option<Res<TransportAllocations>>,
    transport_demand: Option<Res<TransportDemandSnapshot>>,
) {
    let capacity = transport_capacity.as_deref();
    let allocations_res = transport_allocations.as_deref();
    let demand_res = transport_demand.as_deref();

    let mut snapshots = Vec::new();

    for (entity, nation_id, stockpile, workforce, allocations) in nations.iter() {
        let mut stockpile_entries: Vec<_> = stockpile.entries().collect();
        stockpile_entries.sort_by_key(|entry| entry.good);

        let workforce_snapshot = AiWorkforceSnapshot::from_workforce(workforce);
        let allocation_snapshot = AiAllocationSnapshot::from_allocations(allocations);
        let transport_snapshot =
            AiTransportSnapshot::from_resources(entity, capacity, allocations_res, demand_res);

        snapshots.push(AiNationSnapshot {
            entity,
            id: *nation_id,
            stockpile: stockpile_entries,
            workforce: workforce_snapshot,
            allocations: allocation_snapshot,
            transport: transport_snapshot,
        });
    }

    snapshots.sort_by_key(|snapshot| snapshot.entity.index());

    context.turn = turn.current_turn;
    context.phase = turn.phase;
    context.nations = snapshots;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::reservation::ReservationSystem;
    use crate::economy::treasury::Treasury;
    use crate::economy::workforce::{Worker, WorkerHealth};
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{App, World};
    use std::collections::HashMap;

    fn rebuild_context(world: &mut World) {
        let mut system_state: SystemState<(
            ResMut<AiTurnContext>,
            Res<TurnSystem>,
            Query<(
                Entity,
                &NationId,
                &Stockpile,
                &Workforce,
                Option<&Allocations>,
            )>,
            Option<Res<TransportCapacity>>,
            Option<Res<TransportAllocations>>,
            Option<Res<TransportDemandSnapshot>>,
        )> = SystemState::new(world);

        let (context, turn, nations, capacity, allocations, demand) = system_state.get_mut(world);
        populate_ai_turn_context(context, turn, nations, capacity, allocations, demand);
        system_state.apply(world);
    }

    #[test]
    fn builds_snapshot_for_enemy_turn() {
        let mut app = App::new();
        app.world_mut().insert_resource(AiTurnContext::default());
        app.world_mut().insert_resource(TurnSystem {
            current_turn: 3,
            phase: TurnPhase::EnemyTurn,
        });

        app.world_mut()
            .insert_resource(TransportCapacity::default());
        app.world_mut()
            .insert_resource(TransportAllocations::default());
        app.world_mut()
            .insert_resource(TransportDemandSnapshot::default());

        let nation = app
            .world_mut()
            .spawn((
                NationId(1),
                Stockpile::default(),
                Workforce::default(),
                Allocations::default(),
            ))
            .id();

        {
            let mut stockpile = app.world_mut().get_mut::<Stockpile>(nation).unwrap();
            stockpile.add(Good::Grain, 5);
            stockpile.reserve(Good::Grain, 2);
            stockpile.add(Good::Coal, 4);
        }

        {
            let mut workforce = app.world_mut().get_mut::<Workforce>(nation).unwrap();
            workforce.add_untrained(2);
            workforce.workers.push(Worker {
                skill: WorkerSkill::Trained,
                health: WorkerHealth::Healthy,
                food_preference_slot: 0,
            });
            workforce.workers.push(Worker {
                skill: WorkerSkill::Expert,
                health: WorkerHealth::Healthy,
                food_preference_slot: 1,
            });
            workforce.update_labor_pool();
        }

        {
            let mut reservation_system = ReservationSystem::default();
            let mut id_stockpile = Stockpile::default();
            let mut id_workforce = Workforce::default();
            let mut id_treasury = Treasury::default();

            let mut next_id = || {
                reservation_system
                    .try_reserve(
                        Vec::new(),
                        0,
                        0,
                        &mut id_stockpile,
                        &mut id_workforce,
                        &mut id_treasury,
                    )
                    .expect("failed to allocate reservation id")
            };

            let building = app.world_mut().spawn_empty().id();
            let mut allocations = app.world_mut().get_mut::<Allocations>(nation).unwrap();
            allocations
                .production
                .insert((building, Good::Steel), vec![next_id(), next_id()]);
            allocations.recruitment = vec![next_id()];
            allocations
                .training
                .insert(WorkerSkill::Untrained, vec![next_id()]);
            allocations.market_buy_interest.insert(Good::Coal);
            allocations.market_buy_interest.insert(Good::Grain);
            allocations
                .market_sells
                .insert(Good::Steel, vec![next_id()]);
        }

        {
            let mut capacity = app.world_mut().resource_mut::<TransportCapacity>();
            let snapshot = capacity.snapshot_mut(nation);
            snapshot.total = 12;
            snapshot.used = 7;
        }

        {
            let mut allocations = app.world_mut().resource_mut::<TransportAllocations>();
            let nation_alloc = allocations.ensure_nation(nation);
            let slot = nation_alloc.slot_mut(TransportCommodity::Steel);
            slot.requested = 4;
            slot.granted = 2;
        }

        {
            let mut demand = app.world_mut().resource_mut::<TransportDemandSnapshot>();
            let entries = demand.nations.entry(nation).or_default();
            entries.insert(
                TransportCommodity::Steel,
                DemandEntry {
                    supply: 1,
                    demand: 5,
                },
            );
        }

        rebuild_context(app.world_mut());

        let context = app.world().resource::<AiTurnContext>();
        assert_eq!(context.turn(), 3);
        assert_eq!(context.phase(), TurnPhase::EnemyTurn);
        assert!(!context.is_empty());

        let snapshot = context.for_nation(nation).expect("nation snapshot");
        assert_eq!(snapshot.id, NationId(1));

        let grain_entry = snapshot
            .stockpile
            .iter()
            .find(|entry| entry.good == Good::Grain)
            .expect("grain entry");
        assert_eq!(grain_entry.reserved, 2);
        assert_eq!(grain_entry.available, 3);

        let coal_entry = snapshot
            .stockpile
            .iter()
            .find(|entry| entry.good == Good::Coal)
            .expect("coal entry");
        assert_eq!(coal_entry.total, 4);

        assert_eq!(snapshot.workforce.untrained, 2);
        assert_eq!(snapshot.workforce.trained, 1);
        assert_eq!(snapshot.workforce.expert, 1);
        assert_eq!(snapshot.workforce.available_labor, 8);

        assert_eq!(snapshot.allocations.recruitment, 1);
        assert_eq!(snapshot.allocations.production.len(), 1);
        assert_eq!(snapshot.allocations.training.len(), 1);
        assert_eq!(
            snapshot.allocations.market_buy_interest,
            vec![Good::Grain, Good::Coal]
        );
        assert_eq!(snapshot.allocations.market_sells.len(), 1);

        assert_eq!(snapshot.transport.capacity.total, 12);
        assert_eq!(snapshot.transport.capacity.used, 7);
        assert_eq!(snapshot.transport.allocations.len(), 1);
        assert_eq!(
            snapshot.transport.allocations[0].commodity,
            TransportCommodity::Steel
        );
        assert_eq!(snapshot.transport.demand.len(), 1);
        assert_eq!(snapshot.transport.demand[0].demand, 5);
    }

    #[test]
    fn clears_previous_snapshots_when_no_nations() {
        let mut app = App::new();
        app.world_mut().insert_resource(AiTurnContext::default());
        app.world_mut().insert_resource(TurnSystem {
            current_turn: 2,
            phase: TurnPhase::EnemyTurn,
        });

        let nation = app
            .world_mut()
            .spawn((
                NationId(1),
                Stockpile::default(),
                Workforce::default(),
                Allocations::default(),
            ))
            .id();

        rebuild_context(app.world_mut());

        assert!(
            app.world()
                .resource::<AiTurnContext>()
                .for_nation(nation)
                .is_some()
        );

        app.world_mut().entity_mut(nation).despawn();

        {
            let mut turn = app.world_mut().resource_mut::<TurnSystem>();
            turn.phase = TurnPhase::PlayerTurn;
            turn.current_turn = 3;
        }
        rebuild_context(app.world_mut());

        {
            let mut turn = app.world_mut().resource_mut::<TurnSystem>();
            turn.phase = TurnPhase::EnemyTurn;
            turn.current_turn = 4;
        }
        rebuild_context(app.world_mut());

        let context = app.world().resource::<AiTurnContext>();
        assert!(context.is_empty());
        assert_eq!(context.turn(), 4);
        assert_eq!(context.phase(), TurnPhase::EnemyTurn);
    }
}
