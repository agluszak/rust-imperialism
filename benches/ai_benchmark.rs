use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_imperialism::ai::planner::plan_nation;
use rust_imperialism::ai::snapshot::{AiSnapshot, NationSnapshot, CivilianSnapshot, SuggestedDepot, DepotInfo, ImprovableTile, ProspectableTile, MarketSnapshot};
use rust_imperialism::civilians::types::CivilianKind;
use rust_imperialism::economy::goods::Good;
use rust_imperialism::economy::stockpile::StockpileEntry;
use rust_imperialism::map::tiles::TerrainType;
use rust_imperialism::resources::{DevelopmentLevel, ResourceType};
use bevy::prelude::Entity;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet};

fn create_test_snapshot() -> (NationSnapshot, AiSnapshot) {
    let mut stockpile = HashMap::new();
    // Shortage of coal
    stockpile.insert(Good::Coal, StockpileEntry {
        good: Good::Coal,
        total: 5,
        available: 5,
        reserved: 0
    });
    // Surplus of grain
    stockpile.insert(Good::Grain, StockpileEntry {
        good: Good::Grain,
        total: 100,
        available: 100,
        reserved: 0
    });
    // Some steel
    stockpile.insert(Good::Steel, StockpileEntry {
        good: Good::Steel,
        total: 20,
        available: 20,
        reserved: 0
    });

    let mut nation = NationSnapshot {
        entity: Entity::from_bits(1),
        capital_pos: TilePos::new(10, 10),
        treasury: 10000,
        stockpile,
        civilians: Vec::new(),
        connected_tiles: HashSet::new(),
        unconnected_depots: Vec::new(),
        suggested_depots: Vec::new(),
        improvable_tiles: Vec::new(),
        owned_tiles: HashSet::new(),
        depot_positions: HashSet::new(),
        prospectable_tiles: Vec::new(),
        tile_terrain: HashMap::new(),
        technologies: rust_imperialism::economy::technology::Technologies::new(),
        rail_constructions: Vec::new(),
        trade_capacity_total: 100,
        trade_capacity_used: 20,
        buildings: HashMap::new(),
    };

    // Fill with some data
    for x in 0..30 {
        for y in 0..30 {
            let pos = TilePos::new(x, y);
            nation.owned_tiles.insert(pos);
            nation.tile_terrain.insert(pos, TerrainType::Grass);
            if x < 10 {
                nation.connected_tiles.insert(pos);
            }
        }
    }

    // Add civilians
    nation.civilians.push(CivilianSnapshot {
        entity: Entity::from_bits(100),
        kind: CivilianKind::Engineer,
        position: TilePos::new(10, 10),
        has_moved: false,
    });
    nation.civilians.push(CivilianSnapshot {
        entity: Entity::from_bits(101),
        kind: CivilianKind::Farmer,
        position: TilePos::new(10, 11),
        has_moved: false,
    });
    nation.civilians.push(CivilianSnapshot {
        entity: Entity::from_bits(102),
        kind: CivilianKind::Prospector,
        position: TilePos::new(10, 12),
        has_moved: false,
    });

    // Add goals inputs
    nation.suggested_depots.push(SuggestedDepot {
        position: TilePos::new(15, 15),
        covers_count: 5,
        distance_from_capital: 10,
    });

    nation.unconnected_depots.push(DepotInfo {
        position: TilePos::new(5, 15),
        distance_from_capital: 8,
    });

    nation.improvable_tiles.push(ImprovableTile {
        position: TilePos::new(12, 12),
        resource_type: ResourceType::Grain,
        development: DevelopmentLevel::Lv0,
        improver_kind: CivilianKind::Farmer,
        distance_from_capital: 3,
    });

    nation.prospectable_tiles.push(ProspectableTile {
        position: TilePos::new(13, 13),
        distance_from_capital: 4,
    });

    // AiSnapshot
    let mut ai_snapshot = AiSnapshot::default();
    ai_snapshot.occupied_tiles = HashSet::new();

    // Market prices
    let mut prices = HashMap::new();
    prices.insert(Good::Coal, 150); // Expensive coal
    prices.insert(Good::Grain, 80); // Cheap grain
    prices.insert(Good::Steel, 120);
    ai_snapshot.market = MarketSnapshot { prices };

    (nation, ai_snapshot)
}

fn bench_plan_nation(c: &mut Criterion) {
    let (nation, snapshot) = create_test_snapshot();
    c.bench_function("plan_nation", |b| b.iter(|| plan_nation(black_box(&nation), black_box(&snapshot))));
}

criterion_group!(benches, bench_plan_nation);
criterion_main!(benches);
