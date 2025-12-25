use std::collections::{HashMap, HashSet};

use bevy_ecs_tilemap::prelude::TilePos;

use rust_imperialism::ai::planner::plan_nation;
use rust_imperialism::ai::snapshot::AiSnapshot;
use rust_imperialism::ai::{NationPlan, NationSnapshot};
use rust_imperialism::economy::goods::Good;
use rust_imperialism::economy::production::{Buildings, ProductionChoice, ProductionSettings};
use rust_imperialism::economy::technology::Technologies;

fn setup_profitable_market(snapshot: &mut AiSnapshot) {
    snapshot.market.prices.insert(Good::Iron, 50);
    snapshot.market.prices.insert(Good::Coal, 45);
    snapshot.market.prices.insert(Good::Steel, 140);
    snapshot.market.prices.insert(Good::Hardware, 360);
}

fn build_nation_snapshot(entity: bevy::prelude::Entity) -> NationSnapshot {
    NationSnapshot {
        entity,
        capital_pos: TilePos::new(0, 0),
        treasury: 10_000,
        stockpile: HashMap::new(),
        civilians: Vec::new(),
        connected_tiles: HashSet::new(),
        buildings: Some(Buildings::with_all_initial()),
        production_settings: Some(ProductionSettings {
            choice: ProductionChoice::MakeHardware,
            target_output: 0,
        }),
        unconnected_depots: Vec::new(),
        suggested_depots: Vec::new(),
        improvable_tiles: Vec::new(),
        owned_tiles: HashSet::new(),
        depot_positions: HashSet::new(),
        prospectable_tiles: Vec::new(),
        tile_terrain: HashMap::new(),
        technologies: Technologies::default(),
        rail_constructions: Vec::new(),
    }
}

fn plan_for_profitable_hardware() -> NationPlan {
    let mut snapshot = AiSnapshot::default();
    setup_profitable_market(&mut snapshot);

    let nation_entity = bevy::prelude::Entity::from_raw_u32(1).unwrap();
    let nation_snapshot = build_nation_snapshot(nation_entity);
    snapshot
        .nations
        .insert(nation_entity, nation_snapshot.clone());

    plan_nation(&nation_snapshot, &snapshot)
}

#[test]
fn ai_climbs_value_chain_when_hardware_is_profitable() {
    let plan = plan_for_profitable_hardware();

    // Should buy inputs to cover the steel shortfall
    assert!(plan.market_buys.contains(&(Good::Iron, 4)));
    assert!(plan.market_buys.contains(&(Good::Coal, 4)));

    // Should allocate production to transform inputs up the chain
    assert!(
        plan.production_orders
            .iter()
            .any(|p| p.output == Good::Steel && p.qty == 4)
    );
    assert!(
        plan.production_orders
            .iter()
            .any(|p| p.output == Good::Hardware && p.qty == 2)
    );

    // Hardware should be marked for sale once crafted
    assert!(plan.market_sells.contains(&(Good::Hardware, 2)));

    // Metal works should be pointed at the hardware recipe
    assert!(matches!(
        plan.production_choices
            .get(&rust_imperialism::economy::production::BuildingKind::MetalWorks),
        Some(ProductionChoice::MakeHardware)
    ));
}
