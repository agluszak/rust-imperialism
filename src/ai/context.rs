use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::HashMap;

use crate::ai::markers::AiNation;
use crate::economy::allocation::Allocations;
use crate::economy::goods::Good;
use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::nation::{Capital, NationId};
use crate::economy::stockpile::{Stockpile, StockpileEntry};
use crate::economy::transport::{
    CapacitySnapshot, DemandEntry, Depot, Rails, TransportAllocations, TransportCapacity,
    TransportCommodity, TransportDemandSnapshot,
};
use crate::economy::treasury::Treasury;
use crate::economy::workforce::{WorkerSkill, Workforce};
use crate::turn_system::{TurnPhase, TurnSystem};

pub type AiStockpileEntry = StockpileEntry;

/// Identifier for minor nations or city-states the AI can invest in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MinorId(pub u16);

/// Tag describing a macro-level action the AI can pursue this turn.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroTag {
    BuyResource { good: Good },
    UpgradeRail { from: TilePos, to: TilePos },
    InvestMinor { minor: MinorId },
}

/// Target buffer the AI aims to maintain for tradable resources.
pub const RESOURCE_TARGET_DAYS: f32 = 20.0;

pub fn resource_target_days(good: Good) -> f32 {
    if good.is_raw_food() {
        12.0
    } else {
        RESOURCE_TARGET_DAYS
    }
}

/// Candidate macro action generated during the analysis phase.
#[derive(Debug, Clone)]
pub struct MacroActionCandidate {
    pub nation: Entity,
    pub tag: MacroTag,
    pub urgency: f32,
}

/// Turn-scoped list of macro candidates keyed by owning nation.
#[derive(Resource, Debug, Default)]
pub struct TurnCandidates(pub Vec<MacroActionCandidate>);

impl TurnCandidates {
    pub fn for_actor(&self, actor: Entity) -> impl Iterator<Item = &MacroActionCandidate> {
        self.0
            .iter()
            .filter(move |candidate| candidate.nation == actor)
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Snapshot of economic and logistical facts the AI reasons over each turn.
#[derive(Resource, Debug, Clone, Default)]
pub struct BeliefState {
    nations: HashMap<Entity, BeliefNation>,
    turn: u32,
}

impl BeliefState {
    pub fn rebuild<I>(&mut self, turn: u32, entries: I)
    where
        I: Iterator<Item = BeliefNation>,
    {
        self.nations.clear();
        for entry in entries {
            self.nations.insert(entry.entity, entry);
        }
        self.turn = turn;
    }

    pub fn nations(&self) -> impl Iterator<Item = &BeliefNation> {
        self.nations.values()
    }

    pub fn for_entity(&self, entity: Entity) -> Option<&BeliefNation> {
        self.nations.get(&entity)
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }
}

#[derive(Debug, Clone)]
pub struct BeliefNation {
    pub entity: Entity,
    pub id: NationId,
    pub stockpile: Vec<AiStockpileEntry>,
    pub treasury: i64,
}

impl BeliefNation {
    pub fn stockpile_amount(&self, good: Good) -> u32 {
        self.stockpile
            .iter()
            .find(|entry| entry.good == good)
            .map(|entry| entry.total)
            .unwrap_or(0)
    }

    pub fn available_amount(&self, good: Good) -> u32 {
        self.stockpile
            .iter()
            .find(|entry| entry.good == good)
            .map(|entry| entry.available)
            .unwrap_or(0)
    }
}

/// Aggregated market information exposed to scorers and actions.
#[derive(Resource, Debug, Clone, Default)]
pub struct MarketView {
    observations: HashMap<Good, MarketObservation>,
}

impl MarketView {
    pub fn record(&mut self, good: Good, observation: MarketObservation) {
        self.observations.insert(good, observation);
    }

    pub fn price_for(&self, good: Good) -> u32 {
        self.observations
            .get(&good)
            .map(|obs| obs.price)
            .unwrap_or(100)
    }

    pub fn recommended_buy_qty(&self, good: Good, available: u32, desired: u32) -> Option<u32> {
        if available >= desired {
            return None;
        }
        let deficit = desired - available;
        let max_qty = self
            .observations
            .get(&good)
            .map(|obs| obs.demand.saturating_sub(obs.supply))
            .unwrap_or(deficit);
        Some(deficit.min(max_qty.max(1)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketObservation {
    pub price: u32,
    pub supply: u32,
    pub demand: u32,
}

/// Transport analysis derived from depots, capitals, and existing connectivity.
#[derive(Resource, Debug, Default)]
pub struct TransportAnalysis {
    upgrades: HashMap<Entity, Vec<RailUpgradeCandidate>>,
}

impl TransportAnalysis {
    pub fn replace(&mut self, entries: HashMap<Entity, Vec<RailUpgradeCandidate>>) {
        self.upgrades = entries;
    }

    pub fn candidates_for(&self, nation: Entity) -> &[RailUpgradeCandidate] {
        self.upgrades.get(&nation).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RailUpgradeCandidate {
    pub nation: Entity,
    pub from: TilePos,
    pub to: TilePos,
    pub marginal_gain: f32,
}

/// Tracks long-running macro plans and cooldowns so the AI avoids rapid toggling.
#[derive(Resource, Debug, Default)]
pub struct AiPlanLedger {
    cooldowns: HashMap<(Entity, MacroTag), u8>,
    last_turn: Option<u32>,
}

impl AiPlanLedger {
    pub fn apply_cooldown(&mut self, nation: Entity, tag: MacroTag, turns: u8) {
        self.cooldowns.insert((nation, tag), turns.max(1));
    }

    pub fn cooldown_active(&self, nation: Entity, tag: &MacroTag) -> bool {
        self.cooldowns
            .get(&(nation, tag.clone()))
            .map(|value| *value > 0)
            .unwrap_or(false)
    }

    pub fn advance_turn(&mut self, turn: u32) {
        if self.last_turn == Some(turn) {
            return;
        }

        self.last_turn = Some(turn);

        self.cooldowns.retain(|_, remaining| {
            if *remaining > 0 {
                *remaining -= 1;
            }
            *remaining > 0
        });
    }
}

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
    pub market_buys: Vec<AiMarketBuy>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiMarketBuy {
    pub good: Good,
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

            let mut buy_interest: Vec<AiMarketBuy> = allocations
                .market_buys
                .iter()
                .map(|good| AiMarketBuy { good: *good })
                .collect();
            buy_interest.sort_by_key(|entry| entry.good);
            snapshot.market_buys = buy_interest;

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

pub fn update_belief_state_system(
    mut belief: ResMut<BeliefState>,
    turn: Res<TurnSystem>,
    nations: Query<(Entity, &NationId, &Stockpile, &Treasury), With<AiNation>>,
) {
    let mut entries = Vec::new();
    for (entity, nation_id, stockpile, treasury) in nations.iter() {
        let mut stockpile_entries: Vec<_> = stockpile.entries().collect();
        stockpile_entries.sort_by_key(|entry| entry.good);
        entries.push(BeliefNation {
            entity,
            id: *nation_id,
            stockpile: stockpile_entries,
            treasury: treasury.available(),
        });
    }

    entries.sort_by_key(|entry| entry.entity.index());
    belief.rebuild(turn.current_turn, entries.into_iter());
}

pub fn update_market_view_system(
    mut view: ResMut<MarketView>,
    pricing: Res<MarketPriceModel>,
    context: Res<AiTurnContext>,
) {
    view.observations.clear();

    for &good in MARKET_RESOURCES {
        let mut supply = 0;
        let mut demand = 0;

        for nation in context.nations() {
            demand += nation
                .allocations
                .market_buys
                .iter()
                .filter(|entry| entry.good == good)
                .count() as u32;
            supply += nation
                .allocations
                .market_sells
                .iter()
                .filter(|entry| entry.good == good)
                .map(|entry| entry.reserved as u32)
                .sum::<u32>();
        }

        let observation = MarketObservation {
            price: pricing.price_for(good, MarketVolume::new(supply, demand)),
            supply,
            demand,
        };
        view.record(good, observation);
    }
}

pub fn update_transport_analysis_system(
    mut analysis: ResMut<TransportAnalysis>,
    depots: Query<&Depot>,
    rails: Res<Rails>,
    capitals: Query<(Entity, &Capital), With<AiNation>>,
) {
    let mut result: HashMap<Entity, Vec<RailUpgradeCandidate>> = HashMap::new();

    for (nation, capital) in capitals.iter() {
        result.entry(nation).or_default();

        for depot in depots
            .iter()
            .filter(|depot| depot.owner == nation && !depot.connected)
        {
            let dx = (capital.0.x as i32 - depot.position.x as i32).unsigned_abs();
            let dy = (capital.0.y as i32 - depot.position.y as i32).unsigned_abs();
            let distance = (dx + dy) as f32;
            let marginal_gain = (1.0 / (1.0 + distance)).clamp(0.05, 1.0);
            let candidate = RailUpgradeCandidate {
                nation,
                from: capital.0,
                to: depot.position,
                marginal_gain,
            };

            let list = result.entry(nation).or_default();
            if !rails.0.contains(&crate::economy::transport::ordered_edge(
                capital.0,
                depot.position,
            )) {
                list.push(candidate);
            }
        }
    }

    for candidates in result.values_mut() {
        candidates.sort_by(|a, b| b.marginal_gain.total_cmp(&a.marginal_gain));
        candidates.truncate(4);
    }

    analysis.replace(result);
}

pub fn gather_turn_candidates(
    belief: Res<BeliefState>,
    transport: Res<TransportAnalysis>,
    market: Res<MarketView>,
    ledger: Res<AiPlanLedger>,
    mut candidates: ResMut<TurnCandidates>,
    ai_nations: Query<Entity, With<AiNation>>,
) {
    candidates.clear();

    for entity in ai_nations.iter() {
        if let Some(nation) = belief.for_entity(entity) {
            for &good in MARKET_RESOURCES {
                let desired = resource_target_days(good).round() as u32;
                let tag = MacroTag::BuyResource { good };
                if desired == 0 || ledger.cooldown_active(entity, &tag) {
                    continue;
                }

                let available = nation.available_amount(good);
                if let Some(qty) = market.recommended_buy_qty(good, available, desired) {
                    let urgency = (qty as f32 / desired as f32).clamp(0.0, 1.0);
                    candidates.0.push(MacroActionCandidate {
                        nation: entity,
                        tag,
                        urgency,
                    });
                }
            }
        }

        if let Some(candidate) = transport.candidates_for(entity).first() {
            let tag = MacroTag::UpgradeRail {
                from: candidate.from,
                to: candidate.to,
            };
            if !ledger.cooldown_active(entity, &tag) {
                candidates.0.push(MacroActionCandidate {
                    nation: entity,
                    tag,
                    urgency: candidate.marginal_gain,
                });
            }
        }

        // Simple heuristic: periodically consider investing in minors to avoid stagnation
        let minor_tag = MacroTag::InvestMinor {
            minor: MinorId(entity.index() as u16 % 5),
        };
        if !ledger.cooldown_active(entity, &minor_tag) {
            candidates.0.push(MacroActionCandidate {
                nation: entity,
                tag: minor_tag,
                urgency: 0.15,
            });
        }
    }
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
    use crate::ai::context::{BeliefNation, resource_target_days};
    use crate::ai::*;
    use crate::economy::allocation::Allocations;
    use crate::economy::goods::Good;
    use crate::economy::market::MARKET_RESOURCES;
    use crate::economy::nation::NationId;
    use crate::economy::reservation::ReservationSystem;
    use crate::economy::stockpile::Stockpile;
    use crate::economy::transport::{
        DemandEntry, TransportAllocations, TransportCapacity, TransportCommodity,
        TransportDemandSnapshot,
    };
    use crate::economy::treasury::Treasury;
    use crate::economy::workforce::{Worker, WorkerHealth, WorkerSkill, Workforce};
    use crate::turn_system::{TurnPhase, TurnSystem};
    use bevy::ecs::system::SystemState;
    use bevy::prelude::{App, World};

    use super::MarketObservation;

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
    fn gathers_market_candidates_for_multiple_resources() {
        let mut app = App::new();
        app.world_mut().insert_resource(BeliefState::default());
        app.world_mut().insert_resource(MarketView::default());
        app.world_mut()
            .insert_resource(TransportAnalysis::default());
        app.world_mut().insert_resource(AiPlanLedger::default());
        app.world_mut().insert_resource(TurnCandidates::default());

        let nation = app.world_mut().spawn(AiNation(NationId(7))).id();

        {
            let mut stockpile_entries: Vec<AiStockpileEntry> = MARKET_RESOURCES
                .iter()
                .copied()
                .map(|good| {
                    let target = resource_target_days(good).round() as u32;
                    AiStockpileEntry {
                        good,
                        total: target,
                        reserved: 0,
                        available: target,
                    }
                })
                .collect();

            for (good, available) in [(Good::Grain, 4), (Good::Coal, 2)] {
                if let Some(entry) = stockpile_entries
                    .iter_mut()
                    .find(|entry| entry.good == good)
                {
                    entry.total = available;
                    entry.available = available;
                }
            }

            let mut belief = app.world_mut().resource_mut::<BeliefState>();
            belief.rebuild(
                1,
                vec![BeliefNation {
                    entity: nation,
                    id: NationId(7),
                    stockpile: stockpile_entries,
                    treasury: 0,
                }]
                .into_iter(),
            );
        }

        {
            let mut market = app.world_mut().resource_mut::<MarketView>();
            market.record(
                Good::Grain,
                MarketObservation {
                    price: 60,
                    supply: 1,
                    demand: 6,
                },
            );
            market.record(
                Good::Coal,
                MarketObservation {
                    price: 100,
                    supply: 0,
                    demand: 5,
                },
            );
        }

        let mut system_state: SystemState<(
            Res<BeliefState>,
            Res<TransportAnalysis>,
            Res<MarketView>,
            Res<AiPlanLedger>,
            ResMut<TurnCandidates>,
            Query<Entity, With<AiNation>>,
        )> = SystemState::new(app.world_mut());

        {
            let (belief, transport, market, ledger, candidates, ai_nations) =
                system_state.get_mut(app.world_mut());
            gather_turn_candidates(belief, transport, market, ledger, candidates, ai_nations);
            system_state.apply(app.world_mut());
        }

        let candidates = app.world().resource::<TurnCandidates>();
        let mut saw_grain = false;
        let mut saw_coal = false;

        for candidate in candidates.for_actor(nation) {
            match candidate.tag {
                MacroTag::BuyResource { good: Good::Grain } => saw_grain = true,
                MacroTag::BuyResource { good: Good::Coal } => saw_coal = true,
                _ => {}
            }
        }

        assert!(saw_grain);
        assert!(saw_coal);
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
            allocations.market_buys.insert(Good::Coal);
            allocations.market_buys.insert(Good::Grain);
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
            snapshot.allocations.market_buys,
            vec![
                AiMarketBuy { good: Good::Grain },
                AiMarketBuy { good: Good::Coal },
            ]
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

    #[test]
    fn ledger_cooldowns_tick_down_on_new_turn() {
        let mut ledger = AiPlanLedger::default();
        let nation = Entity::from_bits(1);
        let tag = MacroTag::BuyResource { good: Good::Coal };
        ledger.apply_cooldown(nation, tag.clone(), 2);
        assert!(ledger.cooldown_active(nation, &tag));
        ledger.advance_turn(1);
        assert!(ledger.cooldown_active(nation, &tag));
        ledger.advance_turn(2);
        assert!(!ledger.cooldown_active(nation, &tag));
    }
}
