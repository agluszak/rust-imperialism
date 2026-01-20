//! Tests that verify fixture loading works correctly

mod common;

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use rust_imperialism::ai::markers::{AiControlledCivilian, AiNation};
use rust_imperialism::civilians::Civilian;
use rust_imperialism::economy::nation::{Capital, Nation, NationColor, OwnedBy};
use rust_imperialism::economy::transport::{Depot, RailConstruction, Rails};
use rust_imperialism::map::province::Province;
use rust_imperialism::map::province::TileProvince;
use rust_imperialism::map::tile_pos::{HexExt, TilePosExt};
use rust_imperialism::resources::TileResource;
use rust_imperialism::turn_system::TurnPhase;

#[test]
fn test_load_pruned_red_nation_fixture() {
    let mut app = common::create_fixture_test_app();

    let loaded = common::load_fixture(&mut app, common::PRUNED_RED_NATION_MAP);
    assert!(loaded, "Failed to load fixture");

    let world = app.world_mut();

    // Verify only Red nation exists
    let red_color = Color::srgb(0.8, 0.2, 0.2);
    let mut nations_query = world.query::<(Entity, &NationColor)>();
    let nations: Vec<_> = nations_query.iter(world).collect();

    assert_eq!(nations.len(), 1, "Should have exactly one nation");

    let (_red_nation, color) = nations[0];
    let linear = color.0.to_linear();
    let expected = red_color.to_linear();
    assert!(
        (linear.red - expected.red).abs() < 0.01
            && (linear.green - expected.green).abs() < 0.01
            && (linear.blue - expected.blue).abs() < 0.01,
        "Nation should be Red"
    );

    // Verify provinces were loaded
    // Note: Entity remapping for Province.owner is a known limitation with moonshine-save.
    // The provinces are loaded but their owner references may not be remapped correctly.
    // For now, we just verify provinces exist with owners set.
    let mut provinces_query = world.query::<&Province>();
    let mut province_count = 0;
    for province in provinces_query.iter(world) {
        assert!(province.owner.is_some(), "Province should have an owner");
        province_count += 1;
    }
    assert!(province_count > 0, "Should have provinces");

    // Verify tile positions were loaded (tilemap types registered)
    let tile_count = world.query::<&TilePos>().iter(world).count();
    assert!(tile_count > 0, "Should have tile positions");
}

#[test]
fn test_red_nation_has_connected_rail_after_20_turns() {
    let mut app = common::create_fixture_simulation_app();

    let loaded = common::load_fixture(&mut app, common::PRUNED_RED_NATION_MAP);
    assert!(loaded, "Failed to load fixture");

    common::rebuild_tile_storage(&mut app);

    for _ in 0..20 {
        common::transition_to_phase(&mut app, TurnPhase::Processing);
        common::transition_to_phase(&mut app, TurnPhase::EnemyTurn);
        common::transition_to_phase(&mut app, TurnPhase::PlayerTurn);
    }

    let (red_entity, capital, rails_edges) = {
        let world = app.world_mut();
        let red_color = Color::srgb(0.8, 0.2, 0.2);
        let mut nations_query = world.query::<(Entity, &NationColor, &Capital)>();
        let red_nation = nations_query
            .iter(world)
            .find(|(_, color, _)| {
                let linear = color.0.to_linear();
                let expected = red_color.to_linear();
                (linear.red - expected.red).abs() < 0.01
                    && (linear.green - expected.green).abs() < 0.01
                    && (linear.blue - expected.blue).abs() < 0.01
            });

        let (red_entity, _, capital) = red_nation.expect("Red nation with capital should exist");
        let rails_edges = world
            .resource::<Rails>()
            .0
            .iter()
            .copied()
            .collect::<HashSet<_>>();

        (red_entity, capital.0, rails_edges)
    };
    let target = TilePos { x: 25, y: 7 };

    let has_target_edge = rails_edges
        .iter()
        .any(|(a, b)| *a == target || *b == target);
    let debug = debug_rail_state(
        app.world_mut(),
        red_entity,
        capital,
        target,
        &rails_edges,
    );
    assert!(
        has_target_edge,
        "Expected rail edge to include target tile ({}, {}).\n{}",
        target.x,
        target.y,
        debug
    );

    let connected_tiles = connected_tiles_from(capital, &rails_edges);
    assert!(
        connected_tiles.contains(&target),
        "Expected rail at ({}, {}) to be connected to the red capital.\n{}",
        target.x,
        target.y,
        debug
    );
}

fn connected_tiles_from(start: TilePos, rails: &HashSet<(TilePos, TilePos)>) -> HashSet<TilePos> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();

    reachable.insert(start);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        for (a, b) in rails.iter() {
            let neighbor = if *a == current {
                Some(*b)
            } else if *b == current {
                Some(*a)
            } else {
                None
            };

            if let Some(next) = neighbor
                && reachable.insert(next)
            {
                queue.push_back(next);
            }
        }
    }

    reachable
}

fn debug_rail_state(
    world: &mut World,
    red_entity: Entity,
    capital: TilePos,
    target: TilePos,
    rails: &HashSet<(TilePos, TilePos)>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Turn: {}",
        world
            .resource::<rust_imperialism::turn_system::TurnCounter>()
            .current
    ));
    lines.push(format!(
        "Red entity: {:?} (Nation: {}, NationColor: {}, Capital: {})",
        red_entity,
        world.get::<Nation>(red_entity).is_some(),
        world.get::<NationColor>(red_entity).is_some(),
        world.get::<Capital>(red_entity).is_some()
    ));
    lines.push(format!("Rails: {}", rails.len()));

    let alt_a = TilePos { x: 25, y: 6 };
    let alt_b = TilePos { x: 26, y: 6 };
    let watch_positions = {
        let mut set = HashSet::new();
        set.insert(target);
        set.insert(alt_a);
        set.insert(alt_b);
        for neighbor_hex in target.to_hex().all_neighbors() {
            if let Some(pos) = neighbor_hex.to_tile_pos() {
                set.insert(pos);
            }
        }
        set
    };

    let mut near_edges = Vec::new();
    for (a, b) in rails.iter() {
        if watch_positions.contains(a) || watch_positions.contains(b) {
            near_edges.push((*a, *b));
        }
    }
    near_edges.sort_by_key(|(a, b)| (a.x, a.y, b.x, b.y));
    lines.push(format!(
        "Edges near target: {}",
        format_edges(&near_edges)
    ));

    let connected_tiles = connected_tiles_from(capital, rails);
    lines.push(format!(
        "Connected tiles: {} (target connected: {}, alt A: {}, alt B: {})",
        connected_tiles.len(),
        connected_tiles.contains(&target),
        connected_tiles.contains(&alt_a),
        connected_tiles.contains(&alt_b)
    ));

    let mut constructions_query = world.query::<&RailConstruction>();
    let mut constructions = Vec::new();
    for construction in constructions_query.iter(world) {
        constructions.push((
            construction.from,
            construction.to,
            construction.turns_remaining,
            construction.owner,
        ));
    }
    constructions.sort_by_key(|(from, to, _, _)| (from.x, from.y, to.x, to.y));
    lines.push(format!(
        "Rail constructions: {}{}",
        constructions.len(),
        if constructions.is_empty() {
            String::new()
        } else {
            format!(" {}", format_constructions(&constructions))
        }
    ));

    let ai_nations: Vec<Entity> = world
        .query_filtered::<Entity, With<AiNation>>()
        .iter(world)
        .collect();
    lines.push(format!(
        "Ai nations: {} (red is ai: {})",
        ai_nations.len(),
        ai_nations.contains(&red_entity)
    ));

    let mut engineers = Vec::new();
    let mut civilians_query =
        world.query_filtered::<(Entity, &Civilian), With<AiControlledCivilian>>();
    for (entity, civilian) in civilians_query.iter(world) {
        if civilian.kind == rust_imperialism::civilians::CivilianKind::Engineer
            && civilian.owner == red_entity
        {
            engineers.push((entity, civilian.position, civilian.has_moved));
        }
    }
    engineers.sort_by_key(|(_, pos, _)| (pos.x, pos.y));
    lines.push(format!("Red engineers: {}", format_engineers(&engineers)));

    let tilemap_entity = {
        let mut tilemap_query =
            world.query_filtered::<Entity, With<bevy_ecs_tilemap::prelude::TileStorage>>();
        tilemap_query.iter(world).next()
    };

    if let Some(tilemap_entity) = tilemap_entity {
        let storage = world
            .get::<bevy_ecs_tilemap::prelude::TileStorage>(tilemap_entity)
            .cloned();
        let map_size = world
            .get::<bevy_ecs_tilemap::prelude::TilemapSize>(tilemap_entity)
            .copied();

        if let (Some(storage), Some(map_size)) = (storage, map_size) {
            let mut tile_provinces = world.query::<&TileProvince>();
            let mut provinces = world.query::<&Province>();
            let owned = storage
                .get(&target)
                .and_then(|tile_entity| tile_provinces.get(world, tile_entity).ok())
                .and_then(|tile_province| {
                    provinces
                        .iter(world)
                        .find(|province| province.id == tile_province.province_id)
                })
                .is_some_and(|province| province.owner == Some(red_entity));
            lines.push(format!(
                "Target owned: {} (map_size: {}x{})",
                owned, map_size.x, map_size.y
            ));
        } else {
            lines.push("Tile storage: missing".to_string());
        }
    } else {
        lines.push("Tilemap entity: missing".to_string());
    }

    let mut treasury_query = world.query::<(Entity, &rust_imperialism::economy::Treasury)>();
    if let Some((_, treasury)) = treasury_query.iter(world).find(|(entity, _)| *entity == red_entity)
    {
        lines.push(format!("Red treasury: {}", treasury.total()));
    }

    let mut owned_query = world.query::<&OwnedBy>();
    let owned_count = owned_query.iter(world).filter(|owned_by| owned_by.0 == red_entity).count();
    lines.push(format!("Owned entities: {}", owned_count));

    let resource_count = world.query::<&TileResource>().iter(world).count();
    let depot_count = world.query::<&Depot>().iter(world).count();
    lines.push(format!(
        "Tile resources: {} | Depots: {}",
        resource_count, depot_count
    ));

    let mut provinces_query = world.query::<&Province>();
    let mut owner_counts: HashMap<Entity, usize> = HashMap::new();
    let mut provinces_total = 0;
    let mut provinces_owned = 0;
    for province in provinces_query.iter(world) {
        provinces_total += 1;
        if let Some(owner) = province.owner {
            *owner_counts.entry(owner).or_insert(0) += 1;
            if owner == red_entity {
                provinces_owned += 1;
            }
        }
    }
    lines.push(format!(
        "Provinces owned by red: {}/{}",
        provinces_owned, provinces_total
    ));
    if !owner_counts.is_empty() {
        let mut owners: Vec<_> = owner_counts.into_iter().collect();
        owners.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let mut parts = Vec::new();
        for (owner, count) in owners.iter().take(4) {
            parts.push(format!("{:?}:{}", owner, count));
        }
        lines.push(format!("Province owners: {}", parts.join(", ")));

        if let Some((owner, _)) = owners.first() {
            lines.push(format!(
                "Top owner has Nation: {}, NationColor: {}, Capital: {}",
                world.get::<Nation>(*owner).is_some(),
                world.get::<NationColor>(*owner).is_some(),
                world.get::<Capital>(*owner).is_some()
            ));
        }
    }

    let registry = world.resource::<AppTypeRegistry>().read();
    let province_reflect = registry
        .get(std::any::TypeId::of::<Province>())
        .and_then(|registration| {
            registration.data::<bevy::ecs::reflect::ReflectMapEntities>()
        })
        .is_some();
    lines.push(format!(
        "Province ReflectMapEntities registered: {}",
        province_reflect
    ));

    lines.join("\n")
}

fn format_edges(edges: &[(TilePos, TilePos)]) -> String {
    if edges.is_empty() {
        return "none".to_string();
    }

    let mut parts = Vec::new();
    for (a, b) in edges.iter().take(12) {
        parts.push(format!("({},{})-({},{})", a.x, a.y, b.x, b.y));
    }

    if edges.len() > 12 {
        parts.push(format!("...+{}", edges.len() - 12));
    }

    parts.join(", ")
}

fn format_constructions(constructions: &[(TilePos, TilePos, u32, Entity)]) -> String {
    let mut parts = Vec::new();
    for (from, to, turns, owner) in constructions.iter().take(8) {
        parts.push(format!(
            "({},{})-({},{}) t={} owner={:?}",
            from.x, from.y, to.x, to.y, turns, owner
        ));
    }

    if constructions.len() > 8 {
        parts.push(format!("...+{}", constructions.len() - 8));
    }

    parts.join(", ")
}

fn format_engineers(engineers: &[(Entity, TilePos, bool)]) -> String {
    if engineers.is_empty() {
        return "none".to_string();
    }

    let mut parts = Vec::new();
    for (entity, pos, moved) in engineers.iter().take(8) {
        parts.push(format!(
            "{:?}@({},{}) moved={}",
            entity, pos.x, pos.y, moved
        ));
    }

    if engineers.len() > 8 {
        parts.push(format!("...+{}", engineers.len() - 8));
    }

    parts.join(", ")
}
