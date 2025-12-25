//! Nation-level AI planning.
//!
//! This module generates goals for each AI nation based on their current state.
//! Goals are prioritized and then assigned to civilians or executed directly.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::HashMap;

use crate::map::tile_pos::TilePosExt;

use crate::ai::snapshot::{AiSnapshot, NationSnapshot, resource_target_days};
use crate::civilians::types::CivilianKind;
use crate::economy::goods::Good;
use crate::economy::market::MARKET_RESOURCES;
use crate::economy::production::{BuildingKind, ProductionChoice};

/// A goal that a nation wants to accomplish.
#[derive(Debug, Clone)]
pub enum NationGoal {
    /// Buy a resource from the market.
    BuyResource { good: Good, qty: u32, priority: f32 },
    /// Sell excess resources on the market.
    SellResource { good: Good, qty: u32, priority: f32 },
    /// Build a depot on a resource tile.
    BuildDepotAt { tile: TilePos, priority: f32 },
    /// Connect an unconnected depot to the rail network.
    ConnectDepot { tile: TilePos, priority: f32 },
    /// Improve a resource tile.
    ImproveTile {
        tile: TilePos,
        civilian_kind: CivilianKind,
        priority: f32,
    },
    /// Prospect a tile for minerals.
    ProspectTile { tile: TilePos, priority: f32 },
    /// Hire a new civilian.
    HireCivilian { kind: CivilianKind, priority: f32 },
}

impl NationGoal {
    pub fn priority(&self) -> f32 {
        match self {
            NationGoal::BuyResource { priority, .. } => *priority,
            NationGoal::SellResource { priority, .. } => *priority,
            NationGoal::BuildDepotAt { priority, .. } => *priority,
            NationGoal::ConnectDepot { priority, .. } => *priority,
            NationGoal::ImproveTile { priority, .. } => *priority,
            NationGoal::ProspectTile { priority, .. } => *priority,
            NationGoal::HireCivilian { priority, .. } => *priority,
        }
    }
}

/// Output of nation planning: goals and concrete orders.
#[derive(Debug, Clone, Default)]
pub struct NationPlan {
    pub goals: Vec<NationGoal>,
    pub civilian_tasks: HashMap<Entity, CivilianTask>,
    pub market_buys: Vec<(Good, u32)>,
    pub market_sells: Vec<(Good, u32)>,
    pub production_orders: Vec<ProductionOrder>,
    pub production_choices: HashMap<BuildingKind, ProductionChoice>,
    pub civilians_to_hire: Vec<CivilianKind>,
}

#[derive(Debug, Clone)]
pub struct ProductionOrder {
    pub building: Entity,
    pub output: Good,
    pub qty: u32,
}

/// A task assigned to a specific civilian.
#[derive(Debug, Clone)]
pub enum CivilianTask {
    /// Build rail toward a target tile.
    BuildRailTo { target: TilePos },
    /// Build a depot at current location.
    BuildDepot,
    /// Improve the tile at target position.
    ImproveTile { target: TilePos },
    /// Prospect a tile for minerals.
    ProspectTile { target: TilePos },
    /// Move toward a target tile.
    MoveTo { target: TilePos },
    /// Skip turn (no action).
    Idle,
}

/// Civilian hiring targets per type.
const CIVILIAN_TARGETS: &[(CivilianKind, usize)] = &[
    (CivilianKind::Engineer, 2),
    (CivilianKind::Prospector, 2),
    (CivilianKind::Farmer, 2),
    (CivilianKind::Miner, 2),
    (CivilianKind::Rancher, 1),
    (CivilianKind::Forester, 1),
];

/// Market thresholds.
const BUY_SHORTAGE_THRESHOLD: u32 = 12;
const SELL_RESERVE: u32 = 8;
const SELL_MAX_PER_GOOD: u32 = 8;

/// Generate a complete plan for an AI nation.
pub fn plan_nation(nation: &NationSnapshot, snapshot: &AiSnapshot) -> NationPlan {
    let mut plan = NationPlan::default();

    // 1. Generate all goals
    generate_market_goals(nation, snapshot, &mut plan.goals);
    generate_value_added_trade(nation, snapshot, &mut plan);
    generate_infrastructure_goals(nation, &mut plan.goals);
    generate_improvement_goals(nation, &mut plan.goals);
    generate_prospecting_goals(nation, &mut plan.goals);
    generate_hiring_goals(nation, &mut plan.goals);

    // 2. Sort goals by priority (highest first)
    plan.goals.sort_by(|a, b| {
        b.priority()
            .partial_cmp(&a.priority())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3. Assign civilians to goals
    assign_civilians_to_goals(nation, &plan.goals, &mut plan.civilian_tasks);

    // 4. Generate concrete market orders from goals
    for goal in &plan.goals {
        match goal {
            NationGoal::BuyResource { good, qty, .. } => {
                plan.market_buys.push((*good, *qty));
            }
            NationGoal::SellResource { good, qty, .. } => {
                plan.market_sells.push((*good, *qty));
            }
            NationGoal::HireCivilian { kind, .. } => {
                if plan.civilians_to_hire.is_empty() {
                    // Only hire 1 per turn
                    plan.civilians_to_hire.push(*kind);
                }
            }
            _ => {}
        }
    }

    plan
}

fn generate_market_goals(
    nation: &NationSnapshot,
    snapshot: &AiSnapshot,
    goals: &mut Vec<NationGoal>,
) {
    for &good in MARKET_RESOURCES {
        let available = nation.available_amount(good);
        let target = resource_target_days(good).round() as u32;

        // Buy if shortage
        if available < BUY_SHORTAGE_THRESHOLD && available < target {
            let qty = (target - available).min(10);
            let urgency = 1.0 - (available as f32 / target as f32).min(1.0);

            // Adjust priority based on price (lower priority if expensive)
            let base_price = 100u32;
            let current_price = snapshot.market.price_for(good);
            let price_factor = if current_price > base_price * 12 / 10 {
                0.5 // Expensive, reduce priority
            } else if current_price < base_price * 8 / 10 {
                1.2 // Cheap, increase priority
            } else {
                1.0
            };

            goals.push(NationGoal::BuyResource {
                good,
                qty,
                priority: urgency * price_factor * 0.8, // Market goals cap at 0.8
            });
        }

        // Sell if surplus
        if available > target * 2 && available > SELL_RESERVE {
            let sell_qty = (available - target).min(SELL_MAX_PER_GOOD);
            goals.push(NationGoal::SellResource {
                good,
                qty: sell_qty,
                priority: 0.3, // Low priority
            });
        }
    }
}

fn generate_value_added_trade(
    nation: &NationSnapshot,
    snapshot: &AiSnapshot,
    plan: &mut NationPlan,
) {
    let Some(buildings) = nation.buildings.as_ref() else {
        return;
    };

    let Some(steel_mill) = buildings.get(crate::economy::production::BuildingKind::SteelMill)
    else {
        return;
    };

    let Some(metal_works) = buildings.get(crate::economy::production::BuildingKind::MetalWorks)
    else {
        return;
    };

    // Basic price heuristics: only pursue hardware production if the spread is profitable
    let iron_price = snapshot.market.price_for(Good::Iron);
    let coal_price = snapshot.market.price_for(Good::Coal);
    let steel_price = snapshot.market.price_for(Good::Steel);
    let hardware_price = snapshot.market.price_for(Good::Hardware);

    let steel_input_cost = iron_price.saturating_add(coal_price);
    let hardware_input_cost = steel_price.saturating_mul(2);

    if hardware_price <= hardware_input_cost {
        return; // Not profitable to craft hardware right now
    }

    // Target a small batch to ensure AI can progress
    let desired_hardware = metal_works.capacity.min(2);
    if desired_hardware == 0 {
        return;
    }

    // Ensure we have enough steel lined up to feed hardware production
    let steel_needed = desired_hardware.saturating_mul(2);
    let available_steel = nation.available_amount(Good::Steel);
    let steel_shortfall = steel_needed.saturating_sub(available_steel);

    if steel_shortfall > 0 && steel_price > steel_input_cost {
        // Steel is profitable to craft; queue production and buy inputs
        let steel_batches = steel_shortfall.min(steel_mill.capacity);
        plan.production_orders.push(ProductionOrder {
            building: nation.entity,
            output: Good::Steel,
            qty: steel_batches,
        });

        // Buy the raw inputs required to cover the steel shortfall
        let required_iron = steel_batches;
        let required_coal = steel_batches;

        let iron_have = nation.available_amount(Good::Iron);
        let coal_have = nation.available_amount(Good::Coal);

        let iron_buy = required_iron.saturating_sub(iron_have);
        let coal_buy = required_coal.saturating_sub(coal_have);

        if iron_buy > 0 {
            plan.market_buys.push((Good::Iron, iron_buy));
        }

        if coal_buy > 0 {
            plan.market_buys.push((Good::Coal, coal_buy));
        }
    }

    // Queue hardware production using available + incoming steel
    plan.production_choices
        .insert(BuildingKind::MetalWorks, ProductionChoice::MakeHardware);
    plan.production_orders.push(ProductionOrder {
        building: nation.entity,
        output: Good::Hardware,
        qty: desired_hardware,
    });

    // Plan to sell the finished goods once produced
    plan.market_sells.push((Good::Hardware, desired_hardware));
}

fn generate_infrastructure_goals(nation: &NationSnapshot, goals: &mut Vec<NationGoal>) {
    // Add goals for building depots at optimal locations (calculated via greedy set-cover)
    for depot in &nation.suggested_depots {
        // Priority factors:
        // - Coverage: depots that cover more resources get higher priority
        // - Distance: closer depots are preferred
        let coverage_factor = (depot.covers_count as f32 / 7.0).min(1.0);
        let distance_factor = 1.0 / (1.0 + depot.distance_from_capital as f32 * 0.3);
        let priority = (coverage_factor * 0.6 + distance_factor * 0.4).clamp(0.3, 0.85);

        goals.push(NationGoal::BuildDepotAt {
            tile: depot.position,
            priority,
        });
    }

    // Add goals for connecting existing unconnected depots
    for depot in &nation.unconnected_depots {
        // Priority decreases with distance, but existing depots are important
        let priority = (1.2 / (1.0 + depot.distance_from_capital as f32 * 0.1)).clamp(0.4, 0.95);
        goals.push(NationGoal::ConnectDepot {
            tile: depot.position,
            priority,
        });
    }
}

fn generate_improvement_goals(nation: &NationSnapshot, goals: &mut Vec<NationGoal>) {
    for tile in &nation.improvable_tiles {
        // Priority: closer tiles and lower development levels are higher priority
        let distance_factor = 1.0 / (1.0 + tile.distance_from_capital as f32 * 0.1);
        let development_factor = match tile.development {
            crate::resources::DevelopmentLevel::Lv0 => 1.0,
            crate::resources::DevelopmentLevel::Lv1 => 0.7,
            crate::resources::DevelopmentLevel::Lv2 => 0.4,
            crate::resources::DevelopmentLevel::Lv3 => 0.0, // Already max
        };

        let priority = distance_factor * development_factor * 0.6;

        if priority > 0.1 {
            goals.push(NationGoal::ImproveTile {
                tile: tile.position,
                civilian_kind: tile.improver_kind,
                priority,
            });
        }
    }
}

fn generate_prospecting_goals(nation: &NationSnapshot, goals: &mut Vec<NationGoal>) {
    for tile in &nation.prospectable_tiles {
        // Priority: closer tiles are higher priority, prospecting is important for resource discovery
        let distance_factor = 1.0 / (1.0 + tile.distance_from_capital as f32 * 0.15);
        let priority = distance_factor * 0.7; // High priority - finding resources is valuable

        goals.push(NationGoal::ProspectTile {
            tile: tile.position,
            priority,
        });
    }
}

fn generate_hiring_goals(nation: &NationSnapshot, goals: &mut Vec<NationGoal>) {
    for &(kind, target) in CIVILIAN_TARGETS {
        let current = nation.civilian_count(kind);
        if current < target {
            // Check if we can afford it
            let cost = kind.hiring_cost();
            if nation.treasury >= cost {
                goals.push(NationGoal::HireCivilian {
                    kind,
                    priority: 0.4, // Medium priority
                });
            }
        }
    }
}

fn assign_civilians_to_goals(
    nation: &NationSnapshot,
    goals: &[NationGoal],
    tasks: &mut HashMap<Entity, CivilianTask>,
) {
    let mut assigned_goals: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // First pass: Engineers for infrastructure
    for civilian in nation.available_civilians() {
        if civilian.kind != CivilianKind::Engineer {
            continue;
        }

        for (i, goal) in goals.iter().enumerate() {
            if assigned_goals.contains(&i) {
                continue;
            }

            match goal {
                NationGoal::BuildDepotAt { tile, .. } => {
                    // Engineer needs to go to the tile and build a depot
                    if let Some(task) = plan_engineer_depot_task(nation, civilian.position, *tile) {
                        tasks.insert(civilian.entity, task);
                        assigned_goals.insert(i);
                        break;
                    }
                }
                NationGoal::ConnectDepot { tile, .. } => {
                    // Engineer needs to build rail toward an existing depot
                    if let Some(task) = plan_engineer_rail_task(nation, civilian.position, *tile) {
                        tasks.insert(civilian.entity, task);
                        assigned_goals.insert(i);
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    // Second pass: Prospectors for resource discovery
    for civilian in nation.available_civilians() {
        if tasks.contains_key(&civilian.entity) {
            continue;
        }

        if civilian.kind != CivilianKind::Prospector {
            continue;
        }

        for (i, goal) in goals.iter().enumerate() {
            if assigned_goals.contains(&i) {
                continue;
            }

            if let NationGoal::ProspectTile { tile, .. } = goal {
                if civilian.position == *tile || is_adjacent(civilian.position, *tile) {
                    tasks.insert(
                        civilian.entity,
                        CivilianTask::ProspectTile { target: *tile },
                    );
                } else {
                    tasks.insert(civilian.entity, CivilianTask::MoveTo { target: *tile });
                }
                assigned_goals.insert(i);
                break;
            }
        }
    }

    // Third pass: Improvement specialists
    for civilian in nation.available_civilians() {
        if tasks.contains_key(&civilian.entity) {
            continue;
        }

        if !civilian.kind.supports_improvements() {
            continue;
        }

        for (i, goal) in goals.iter().enumerate() {
            if assigned_goals.contains(&i) {
                continue;
            }

            if let NationGoal::ImproveTile {
                tile,
                civilian_kind,
                ..
            } = goal
                && *civilian_kind == civilian.kind
            {
                if civilian.position == *tile || is_adjacent(civilian.position, *tile) {
                    tasks.insert(civilian.entity, CivilianTask::ImproveTile { target: *tile });
                } else {
                    tasks.insert(civilian.entity, CivilianTask::MoveTo { target: *tile });
                }
                assigned_goals.insert(i);
                break;
            }
        }
    }

    // Default: unassigned civilians are idle
    for civilian in nation.available_civilians() {
        tasks.entry(civilian.entity).or_insert(CivilianTask::Idle);
    }
}

/// Plan an engineer task to build a depot at a target tile.
fn plan_engineer_depot_task(
    nation: &NationSnapshot,
    engineer_pos: TilePos,
    target: TilePos,
) -> Option<CivilianTask> {
    // 1. Find the bridgehead: the connected tile closest to target
    let bridgehead = nation
        .connected_tiles
        .iter()
        .min_by_key(|t| {
            (
                t.to_hex().distance_to(target.to_hex()),
                if **t == engineer_pos { 0 } else { 1 }, // Prefer current tile if tied for distance
                t.x,                                     // Consistent tie-breaking
                t.y,
            )
        })
        .copied()?;

    // 2. If we are not at the bridgehead, teleport there
    if engineer_pos != bridgehead {
        return Some(CivilianTask::MoveTo { target: bridgehead });
    }

    // 3. We are at the bridgehead. If it's the target, build the depot.
    if bridgehead == target {
        if can_build_depot_here(target, nation) {
            return Some(CivilianTask::BuildDepot);
        }
        return None;
    }

    // 4. We are at the bridgehead but not at the target. Build rail towards target.
    if let Some(next_tile) = find_step_toward(bridgehead, target, &nation.owned_tiles) {
        // next_tile MUST be unconnected if bridgehead was the closest connected tile.
        if !nation.connected_tiles.contains(&next_tile) {
            // Check if this rail segment is already being built
            if is_rail_being_built(bridgehead, next_tile, nation) {
                return None;
            }

            if can_build_rail_between(bridgehead, next_tile, nation) {
                return Some(CivilianTask::BuildRailTo { target: next_tile });
            }
        } else {
            // Should not happen if bridgehead logic is correct, but for safety:
            return Some(CivilianTask::MoveTo { target: next_tile });
        }
    }

    None
}

/// Plan an engineer task to build rail connecting to an existing depot.
fn plan_engineer_rail_task(
    nation: &NationSnapshot,
    engineer_pos: TilePos,
    depot_pos: TilePos,
) -> Option<CivilianTask> {
    // 1. Find the bridgehead: the connected tile closest to depot_pos
    let bridgehead = nation
        .connected_tiles
        .iter()
        .min_by_key(|t| {
            (
                t.to_hex().distance_to(depot_pos.to_hex()),
                if **t == engineer_pos { 0 } else { 1 }, // Prefer current tile if tied for distance
                t.x,                                     // Consistent tie-breaking
                t.y,
            )
        })
        .copied()?;

    // 2. If we are not at the bridgehead, teleport there
    if engineer_pos != bridgehead {
        return Some(CivilianTask::MoveTo { target: bridgehead });
    }

    // 3. We are at the bridgehead. If it's the depot_pos, we are done (connected).
    if bridgehead == depot_pos {
        return None;
    }

    // 4. We are at the bridgehead but not at the depot. Build rail towards depot.
    if let Some(next_tile) = find_step_toward(bridgehead, depot_pos, &nation.owned_tiles) {
        // next_tile MUST be unconnected if bridgehead was the closest connected tile.
        if !nation.connected_tiles.contains(&next_tile) {
            // Check if this rail segment is already being built
            if is_rail_being_built(bridgehead, next_tile, nation) {
                return None;
            }

            if can_build_rail_between(bridgehead, next_tile, nation) {
                return Some(CivilianTask::BuildRailTo { target: next_tile });
            }
        } else {
            // Should not happen if bridgehead logic is correct, but for safety:
            return Some(CivilianTask::MoveTo { target: next_tile });
        }
    }

    None
}

/// Find the next step from `from` toward `to`, constrained to `allowed_tiles`.
fn find_step_toward(
    from: TilePos,
    to: TilePos,
    allowed_tiles: &std::collections::HashSet<TilePos>,
) -> Option<TilePos> {
    use crate::map::tile_pos::HexExt;

    let from_hex = from.to_hex();
    let to_hex = to.to_hex();

    // Find the neighbor that minimizes distance to target
    from_hex
        .all_neighbors()
        .into_iter()
        .filter_map(|hex| hex.to_tile_pos())
        .filter(|pos| allowed_tiles.contains(pos))
        .min_by_key(|pos| {
            (
                pos.to_hex().distance_to(to_hex),
                pos.x, // Consistent tie-breaking
                pos.y,
            )
        })
}

/// Check if two positions are adjacent on the hex grid.
fn is_adjacent(a: TilePos, b: TilePos) -> bool {
    a.to_hex().distance_to(b.to_hex()) == 1
}

/// Check if a rail is currently under construction between two tiles.
fn is_rail_being_built(a: TilePos, b: TilePos, nation: &NationSnapshot) -> bool {
    let edge = crate::economy::transport::ordered_edge(a, b);
    nation
        .rail_constructions
        .iter()
        .any(|rc| crate::economy::transport::ordered_edge(rc.from, rc.to) == edge)
}

/// Check if a rail can be built on a tile given the nation's technologies.
fn can_build_rail_here(tile_pos: TilePos, nation: &NationSnapshot) -> bool {
    nation
        .tile_terrain
        .get(&tile_pos)
        .map(|terrain| {
            crate::economy::transport::can_build_rail_on_terrain(terrain, &nation.technologies).0
        })
        .unwrap_or(false)
}

/// Check if a rail can be built between two adjacent tiles.
/// Both tiles must support rail construction given the nation's technologies.
fn can_build_rail_between(from: TilePos, to: TilePos, nation: &NationSnapshot) -> bool {
    can_build_rail_here(from, nation) && can_build_rail_here(to, nation)
}

/// Check if a depot can be built on a tile.
fn can_build_depot_here(tile_pos: TilePos, nation: &NationSnapshot) -> bool {
    nation
        .tile_terrain
        .get(&tile_pos)
        .map(crate::economy::transport::can_build_depot_on_terrain)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_priority_ordering() {
        let goals = vec![
            NationGoal::BuyResource {
                good: Good::Coal,
                qty: 5,
                priority: 0.5,
            },
            NationGoal::ConnectDepot {
                tile: TilePos::new(0, 0),
                priority: 0.8,
            },
            NationGoal::HireCivilian {
                kind: CivilianKind::Engineer,
                priority: 0.3,
            },
        ];

        let mut sorted = goals.clone();
        sorted.sort_by(|a, b| {
            b.priority()
                .partial_cmp(&a.priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        assert!(sorted[0].priority() > sorted[1].priority());
        assert!(sorted[1].priority() > sorted[2].priority());
    }

    #[test]
    fn test_engineer_moves_directly_to_connected_tile() {
        use std::collections::HashSet;

        // Engineer is far from connected tiles, should move directly to closest one
        let engineer_pos = TilePos::new(10, 10);
        let connected_tile = TilePos::new(5, 5);
        let target = TilePos::new(8, 8);

        let mut connected_tiles = HashSet::new();
        connected_tiles.insert(connected_tile);

        let mut owned_tiles = HashSet::new();
        owned_tiles.insert(engineer_pos);
        owned_tiles.insert(connected_tile);
        owned_tiles.insert(target);

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            capital_pos: TilePos::new(0, 0),
            treasury: 1000,
            stockpile: HashMap::new(),
            civilians: vec![],
            connected_tiles,
            unconnected_depots: vec![],
            suggested_depots: vec![],
            improvable_tiles: vec![],
            owned_tiles,
            depot_positions: HashSet::new(),
            prospectable_tiles: vec![],
            tile_terrain,
            technologies: crate::economy::technology::Technologies::new(),
            rail_constructions: vec![],
        };

        let task = plan_engineer_depot_task(&snapshot, engineer_pos, target);

        // Should move directly to connected tile, not incremental step
        assert!(matches!(task, Some(CivilianTask::MoveTo { target: t }) if t == connected_tile));
    }

    #[test]
    fn test_engineer_builds_rail_when_on_connected_tile() {
        use std::collections::HashSet;

        // Engineer is on a connected tile, should build rail toward target
        let engineer_pos = TilePos::new(5, 5);
        let target = TilePos::new(8, 8);
        let next_step = TilePos::new(6, 5); // Adjacent tile toward target

        let mut connected_tiles = HashSet::new();
        connected_tiles.insert(engineer_pos);

        let mut owned_tiles = HashSet::new();
        owned_tiles.insert(engineer_pos);
        owned_tiles.insert(next_step);
        owned_tiles.insert(target);

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            capital_pos: TilePos::new(0, 0),
            treasury: 1000,
            stockpile: HashMap::new(),
            civilians: vec![],
            connected_tiles,
            unconnected_depots: vec![],
            suggested_depots: vec![],
            improvable_tiles: vec![],
            owned_tiles,
            depot_positions: HashSet::new(),
            prospectable_tiles: vec![],
            tile_terrain,
            technologies: crate::economy::technology::Technologies::new(),
            rail_constructions: vec![],
        };

        let task = plan_engineer_depot_task(&snapshot, engineer_pos, target);

        // Should build rail to adjacent tile toward target
        assert!(matches!(task, Some(CivilianTask::BuildRailTo { target: t }) if t == next_step));
    }

    #[test]
    fn test_engineer_bridgehead_loop() {
        use std::collections::HashSet;

        // Two connected tiles (0,0) and (0,1) equally close to target (1,1)
        // Capital at (0,0), another connected tile at (0,1).
        // Hub at (1,1).
        let capital_pos = TilePos::new(0, 0);
        let pos_0_1 = TilePos::new(0, 1);
        let target = TilePos::new(1, 1);

        let mut connected_tiles = HashSet::new();
        connected_tiles.insert(capital_pos);
        connected_tiles.insert(pos_0_1);

        let mut owned_tiles = HashSet::new();
        owned_tiles.insert(capital_pos);
        owned_tiles.insert(pos_0_1);
        owned_tiles.insert(target);

        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            capital_pos,
            treasury: 1000,
            stockpile: HashMap::new(),
            civilians: vec![],
            connected_tiles,
            unconnected_depots: vec![],
            suggested_depots: vec![],
            improvable_tiles: vec![],
            owned_tiles,
            depot_positions: HashSet::new(),
            prospectable_tiles: vec![],
            tile_terrain,
            technologies: crate::economy::technology::Technologies::new(),
            rail_constructions: vec![],
        };

        // If bridgehead logic picks (0,0) as better than (0,1) due to tie-breaking,
        // and engineer is at (0,1), it will MoveTo (0,0).
        let task = plan_engineer_rail_task(&snapshot, pos_0_1, target);

        // This is fine if it leads to progress.
        // But if then it tries to move from (0,0) to (0,1), it's a loop.
        if let Some(CivilianTask::MoveTo { target: t }) = task
            && t == capital_pos
        {
            // Now check what happens at (0,0)
            let task2 = plan_engineer_rail_task(&snapshot, capital_pos, target);
            // If task2 is MoveTo(0,1), we have a loop!
            assert!(
                !matches!(task2, Some(CivilianTask::MoveTo { target: next }) if next == pos_0_1),
                "Loop detected: (0,1) -> (0,0) -> (0,1)"
            );
        }
    }
}
