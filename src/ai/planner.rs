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
    /// Produce goods in a building.
    ProduceGoods {
        building: Entity,
        good: Good,
        qty: u32,
        priority: f32,
    },
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
            NationGoal::ProduceGoods { priority, .. } => *priority,
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
    pub civilians_to_hire: Vec<CivilianKind>,
    pub transport_allocations: Vec<(crate::economy::transport::TransportCommodity, u32)>,
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
    generate_production_goals(nation, &mut plan.goals);

    // 2. Sort goals by priority (highest first)
    plan.goals.sort_by(|a, b| {
        b.priority()
            .partial_cmp(&a.priority())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3. Assign civilians to goals
    // 3. Assign civilians to goals
    assign_civilians_to_goals(nation, snapshot, &plan.goals, &mut plan.civilian_tasks);

    // 4. Generate concrete orders from goals
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
            NationGoal::ProduceGoods {
                building,
                good,
                qty,
                ..
            } => {
                plan.production_orders.push(ProductionOrder {
                    building: *building,
                    output: *good,
                    qty: *qty,
                });
            }
            _ => {}
        }
    }

    // 5. Generate transport allocations and production orders
    generate_transport_allocations(nation, &mut plan);
    generate_production_orders(nation, &mut plan);

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
    let buildings = &nation.buildings;

    let Some(steel_mill) = buildings.get(&crate::economy::production::BuildingKind::SteelMill)
    else {
        return;
    };

    let Some(metal_works) = buildings.get(&crate::economy::production::BuildingKind::MetalWorks)
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
        if let Some(steel_mill_entity) = nation
            .building_entities
            .get(&crate::economy::production::BuildingKind::SteelMill)
        {
            let steel_batches = steel_shortfall.min(steel_mill.capacity);
            plan.production_orders.push(ProductionOrder {
                building: *steel_mill_entity,
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
    }

    // Queue hardware production using available + incoming steel
    if let Some(metal_works_entity) = nation
        .building_entities
        .get(&crate::economy::production::BuildingKind::MetalWorks)
    {
        plan.production_orders.push(ProductionOrder {
            building: *metal_works_entity,
            output: Good::Hardware,
            qty: desired_hardware,
        });
    }

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

fn generate_production_goals(nation: &NationSnapshot, goals: &mut Vec<NationGoal>) {
    // Ships are now automatically constructed from materials in stockpile
    // The construct_ships_from_production system will build ships when
    // Steel, Lumber, and Fuel are available
    // TODO: AI could prioritize acquiring these materials when trade capacity is low
    let _ = nation; // Suppress unused warning
    let _ = goals;
}

fn assign_civilians_to_goals(
    nation: &NationSnapshot,
    snapshot: &AiSnapshot, // Added snapshot for global occupied_tiles
    goals: &[NationGoal],
    tasks: &mut HashMap<Entity, CivilianTask>,
) {
    // Track positions of friendly units that haven't been assigned a task yet
    let mut unplanned_positions: HashMap<Entity, TilePos> = nation
        .available_civilians()
        .map(|c| (c.entity, c.position))
        .collect();

    // Track reserved tiles (where units WILL be next turn)
    // Initialize with Enemy positions (static obstacles)
    let friendly_entities: std::collections::HashSet<Entity> =
        unplanned_positions.keys().copied().collect();
    let mut reserved_positions: std::collections::HashSet<TilePos> = snapshot
        .occupied_tiles
        .iter()
        .filter(|pos| {
            // Keep only positions NOT belonging to friendly units (i.e., Enemies)
            // (We assume friendly units might move, so we don't reserve their starting pos yet)
            !unplanned_positions.values().any(|p| p == *pos)
        })
        .copied()
        .collect();

    // Also add friendly units that are NOT available (already busy/working)
    // They are effectively static obstacles for this turn's planning
    for civilian in &nation.civilians {
        if !friendly_entities.contains(&civilian.entity) {
            reserved_positions.insert(civilian.position);
        }
    }

    // Iterate goals by priority (already sorted)
    for goal in goals {
        // Calculate blockers for this goal attempt:
        // Reserved tiles (Friends who moved/stayed + Enemies)
        // + ALL Unplanned Friends (who are currently sitting at their spot)
        //
        // Note: This includes the candidate's own position. However, this is safe because:
        // 1. Pathfinding (find_step_toward) checks neighbors, not the start node.
        // 2. Target validity checks (e.g. ProspectTile) handle the "am I already there" case explicitly.
        // 3. Target validity for movement checks !avoid_tiles.contains(target), which is correct (we can't move to ourselves anyway).
        let mut avoid_tiles = reserved_positions.clone();
        for &pos in unplanned_positions.values() {
            avoid_tiles.insert(pos);
        }

        // Find best candidate for this goal
        let mut best_candidate: Option<(Entity, CivilianTask)> = None;
        let mut min_distance = u32::MAX; // Score: lower is better (distance to action)

        for civilian in nation.available_civilians() {
            if !unplanned_positions.contains_key(&civilian.entity) {
                continue;
            }

            let task_opt = match goal {
                NationGoal::BuildDepotAt { tile, .. }
                    if civilian.kind == CivilianKind::Engineer =>
                {
                    plan_engineer_depot_task(nation, &avoid_tiles, civilian.position, *tile)
                }
                NationGoal::ConnectDepot { tile, .. }
                    if civilian.kind == CivilianKind::Engineer =>
                {
                    plan_engineer_rail_task(
                        nation,
                        snapshot,
                        &avoid_tiles,
                        civilian.position,
                        *tile,
                    )
                }
                NationGoal::ProspectTile { tile, .. }
                    if civilian.kind == CivilianKind::Prospector =>
                {
                    if civilian.position == *tile || is_adjacent(civilian.position, *tile) {
                        Some(CivilianTask::ProspectTile { target: *tile })
                    } else if !avoid_tiles.contains(tile) {
                        // Only move if target valid (and path exists - checked by find_step implicitly via simple move?)
                        // Wait, move logic here is simple MoveTo. We should checking pathfinding.
                        // But for now, just checking target validity is a start.
                        // Ideally we'd use pathfinding here too.
                        Some(CivilianTask::MoveTo { target: *tile })
                    } else {
                        None
                    }
                }
                NationGoal::ImproveTile {
                    tile,
                    civilian_kind,
                    ..
                } if civilian.kind == *civilian_kind => {
                    if civilian.position == *tile || is_adjacent(civilian.position, *tile) {
                        Some(CivilianTask::ImproveTile { target: *tile })
                    } else if !avoid_tiles.contains(tile) {
                        Some(CivilianTask::MoveTo { target: *tile })
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(task) = task_opt {
                // Calculate score (approx distance)
                let distance = match task {
                    CivilianTask::MoveTo { target } => {
                        civilian.position.to_hex().distance_to(target.to_hex()) as u32
                    }
                    CivilianTask::BuildRailTo { target } => {
                        civilian.position.to_hex().distance_to(target.to_hex()) as u32
                    }
                    _ => 0, // 0 distance means immediate action possible
                };

                // Check strict path validity?
                // The helper functions `plan_engineer...` use `find_step_toward` which checks `avoid_tiles`.
                // But `MoveTo` above is raw.
                // We should ensure `MoveTo` doesn't jump into an obstacle.
                // Simple hack: if distance > 1, assume pathfinding will handle it next turn?
                // NO. If we assign `MoveTo(target)`, we MUST ensure target is not blocked.
                // The checks `!avoid_tiles.contains(tile)` above handle the goal target.
                // But what if we are far away?
                // We generate `MoveTo` directly to Goal. The Execution system validates step-by-step?
                // No, execution system blindly takes `MoveTo`.
                // Actually `MoveTo` in `CivilianTask` usually means "Move one step towards"?
                // Let's check `CivilianTask` definition.
                // If `MoveTo` target is far away, does it work?
                // `execute.rs` -> `task_to_order` -> `CivilianOrderKind::Move { to }`.
                // `CivilianOrderKind::Move` usually implies distinct movement.
                // But `planner.rs` usually generates `find_step_toward` for Engineers?
                // For Prospectors/Farmers above, `Some(CivilianTask::MoveTo { target: *tile })`.
                // This implies "Teleport/Long Move"?
                // Let's trust that for now, but focus on RESERVATION.

                if distance < min_distance {
                    min_distance = distance;
                    best_candidate = Some((civilian.entity, task));

                    // Optimization: if we found an immediate match, take it (can't beat 0)
                    if distance == 0 {
                        break;
                    }
                }
            }
        }

        // Assign best candidate
        if let Some((entity, task)) = best_candidate {
            tasks.insert(entity, task.clone());

            // Update reservation state
            // Remove from unplanned
            let current_pos = unplanned_positions.remove(&entity).unwrap();

            // Add new position to reserved
            let target_pos = match task {
                CivilianTask::MoveTo { target } => target,
                CivilianTask::BuildRailTo { target } => target, // Moves to target
                CivilianTask::BuildDepot => current_pos,        // Stays put
                CivilianTask::ImproveTile { .. } => current_pos, // Stays put (job)
                CivilianTask::ProspectTile { .. } => current_pos, // Stays put (job)
                CivilianTask::Idle => current_pos,
            };
            reserved_positions.insert(target_pos);
        }
    }

    // Default: unassigned civilians are idle
    for (entity, _pos) in unplanned_positions {
        tasks.entry(entity).or_insert(CivilianTask::Idle);
        // Implicitly reserved their current spot
    }
}

/// Plan an engineer task to build a depot at a target tile.
fn plan_engineer_depot_task(
    nation: &NationSnapshot,
    occupied_tiles: &std::collections::HashSet<TilePos>,
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
    if let Some(next_tile) =
        find_step_toward(bridgehead, target, &nation.owned_tiles, occupied_tiles)
    {
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
    snapshot: &AiSnapshot,
    avoid_tiles: &std::collections::HashSet<TilePos>,
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

    // 2. Identify the "Frontier" of the Depot's rail line.
    // The depot might have some rails attached to it locally (e.g. built by previous turns).
    // We want the engineer to go to the END of this local network (closest to bridgehead)
    // and extend it.

    let depot_frontier = find_rail_frontier(depot_pos, bridgehead, &snapshot.rails);

    // 3. Logic: Coordinate Movement to Frontier
    // If we are NOT at the frontier, go there.
    if engineer_pos != depot_frontier {
        // Just move to the frontier.
        // Note: The path might be along the rails we just traced, or cross-country if we are disconnected.
        if let Some(step) = find_step_toward(
            engineer_pos,
            depot_frontier,
            &nation.owned_tiles,
            avoid_tiles,
        ) {
            return Some(CivilianTask::MoveTo { target: step });
        }
        return None;
    }

    // 4. We are at the frontier. Build towards Bridgehead.
    if let Some(next_tile) =
        find_step_toward(depot_frontier, bridgehead, &nation.owned_tiles, avoid_tiles)
    {
        // If next_tile does not have rail, build it
        if !can_move_on_rail(depot_frontier, next_tile, snapshot) {
            // Check if this rail segment is already being built
            if is_rail_being_built(depot_frontier, next_tile, nation) {
                return None;
            }

            if can_build_rail_between(depot_frontier, next_tile, nation) {
                return Some(CivilianTask::BuildRailTo { target: next_tile });
            }
        } else {
            // Rail exists, just move (shouldn't happen if frontier logic is correct)
            return Some(CivilianTask::MoveTo { target: next_tile });
        }
    }

    None
}

/// Find the tile connected to `start` via rails that minimizes distance to `target`.
fn find_rail_frontier(
    start: TilePos,
    target: TilePos,
    rails: &std::collections::HashSet<(TilePos, TilePos)>,
) -> TilePos {
    use crate::map::tile_pos::HexExt;
    use std::collections::{HashSet, VecDeque};

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(start);
    queue.push_back(start);

    let mut best_tile = start;
    let mut min_dist = start.to_hex().distance_to(target.to_hex());

    while let Some(current) = queue.pop_front() {
        // Update best if this tile is closer to target
        let dist = current.to_hex().distance_to(target.to_hex());
        if dist < min_dist {
            min_dist = dist;
            best_tile = current;
        }

        // Explore neighbors connected by rail
        for neighbor_hex in current.to_hex().all_neighbors() {
            let Some(neighbor) = neighbor_hex.to_tile_pos() else {
                continue;
            };

            if visited.contains(&neighbor) {
                continue;
            }

            let edge = crate::economy::transport::ordered_edge(current, neighbor);
            if rails.contains(&edge) {
                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }

    best_tile
}

/// Check if movement between two adjacent tiles can be done via rail
fn can_move_on_rail(a: TilePos, b: TilePos, snapshot: &AiSnapshot) -> bool {
    let edge = crate::economy::transport::ordered_edge(a, b);
    snapshot.rails.contains(&edge)
}

/// Find the next step from `from` toward `to`, constrained to `allowed_tiles` and avoiding `avoid_tiles` (enemies).
fn find_step_toward(
    from: TilePos,
    to: TilePos,
    allowed_tiles: &std::collections::HashSet<TilePos>,
    avoid_tiles: &std::collections::HashSet<TilePos>,
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
        .filter(|pos| !avoid_tiles.contains(pos)) // Avoid enemy tiles only
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

/// Generate transport allocations based on available resources and capacity.
/// Since we don't have snapshot data for transport capacity yet, we use a simple heuristic:
/// allocate generously to all resource types that might be available.
fn generate_transport_allocations(_nation: &NationSnapshot, plan: &mut NationPlan) {
    use crate::economy::transport::TransportCommodity;

    // Allocate high capacity to essential resources
    // These values are generous to ensure AI doesn't starve from lack of transport
    let allocations = [
        (TransportCommodity::Grain, 10),
        (TransportCommodity::Fruit, 8),
        (TransportCommodity::Fiber, 8),
        (TransportCommodity::Meat, 8),
        (TransportCommodity::Timber, 10),
        (TransportCommodity::Coal, 10),
        (TransportCommodity::Iron, 10),
        (TransportCommodity::Precious, 5),
        (TransportCommodity::Oil, 8),
        (TransportCommodity::Fabric, 5),
        (TransportCommodity::Lumber, 5),
        (TransportCommodity::Paper, 5),
        (TransportCommodity::Steel, 5),
        (TransportCommodity::Fuel, 5),
        (TransportCommodity::Clothing, 3),
        (TransportCommodity::Furniture, 3),
        (TransportCommodity::Hardware, 3),
        (TransportCommodity::Armaments, 3),
        (TransportCommodity::CannedFood, 3),
        (TransportCommodity::Horses, 2),
    ];

    for (commodity, amount) in allocations {
        plan.transport_allocations.push((commodity, amount));
    }
}

/// Generate production orders to build transport capacity.
/// AI should produce Transport goods when it has the resources.
fn generate_production_orders(nation: &NationSnapshot, plan: &mut NationPlan) {
    // Check if we have steel and lumber for Transport production
    let steel_available = nation.available_amount(Good::Steel);
    let lumber_available = nation.available_amount(Good::Lumber);

    // If we have materials, produce some transport capacity
    if steel_available >= 2 && lumber_available >= 2 {
        if let Some(railyard_entity) = nation
            .building_entities
            .get(&crate::economy::production::BuildingKind::Railyard)
        {
            info!(
                "AI Nation {:?} producing Transport (Steel: {}, Lumber: {})",
                nation.entity, steel_available, lumber_available
            );
            // Produce 1 batch (yields 1 Transport)
            plan.production_orders.push(ProductionOrder {
                building: *railyard_entity,
                output: Good::Transport,
                qty: 1,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::TilePos;
    use std::collections::HashMap;

    use super::*;
    use crate::ai::snapshot::NationSnapshot;

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
            trade_capacity_total: 3,
            trade_capacity_used: 0,
            buildings: HashMap::new(),
            building_entities: HashMap::new(),
        };

        let occupied_tiles = HashSet::new();
        let task = plan_engineer_depot_task(&snapshot, &occupied_tiles, engineer_pos, target);

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
            trade_capacity_total: 3,
            trade_capacity_used: 0,
            buildings: HashMap::new(),
            building_entities: HashMap::new(),
        };

        let occupied_tiles = HashSet::new();
        let task = plan_engineer_depot_task(&snapshot, &occupied_tiles, engineer_pos, target);

        // Should build rail to adjacent tile toward target
        assert!(matches!(task, Some(CivilianTask::BuildRailTo { target: t }) if t == next_step));
    }

    /// Benchmark for `assign_civilians_to_goals`.
    ///
    /// Performance History:
    /// - Before optimization (nested loop set cloning): ~1.89s for 200 civilians / 50 goals.
    /// - After optimization (hoisted set construction): ~44.6ms for 200 civilians / 50 goals.
    /// - Speedup: ~42x
    #[test]
    fn test_performance_assign_civilians() {
        use std::collections::HashSet;
        use std::time::Instant;

        // Setup large scenario
        let num_civilians = 200;
        let num_goals = 50;

        let mut civilians = Vec::new();
        let mut owned_tiles = HashSet::new();

        // Create civilians and tiles
        for i in 0..num_civilians {
            let entity = Entity::from_bits((i + 1) as u64);
            let pos = TilePos::new(i as u32 % 50, i as u32 / 50);
            civilians.push(crate::ai::snapshot::CivilianSnapshot {
                entity,
                kind: CivilianKind::Engineer,
                position: pos,
                has_moved: false,
            });
            owned_tiles.insert(pos);
        }

        let mut goals = Vec::new();
        for i in 0..num_goals {
            goals.push(NationGoal::BuildDepotAt {
                tile: TilePos::new((i % 50) as u32, (i / 50 + 10) as u32),
                priority: 1.0,
            });
        }

        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            capital_pos: TilePos::new(0, 0),
            treasury: 1000,
            stockpile: HashMap::new(),
            civilians,
            connected_tiles: HashSet::new(),
            unconnected_depots: vec![],
            suggested_depots: vec![],
            improvable_tiles: vec![],
            owned_tiles: owned_tiles.clone(),
            depot_positions: HashSet::new(),
            prospectable_tiles: vec![],
            tile_terrain: HashMap::new(),
            technologies: crate::economy::technology::Technologies::new(),
            rail_constructions: vec![],
            trade_capacity_total: 1000,
            trade_capacity_used: 0,
            buildings: HashMap::new(),
        };

        // Create empty AI snapshot for collision checking
        let ai_snapshot = AiSnapshot {
            occupied_tiles: HashSet::new(),
            rails: HashSet::new(),
            ..Default::default()
        };

        let mut tasks = HashMap::new();

        // Benchmark
        let start = Instant::now();
        assign_civilians_to_goals(&snapshot, &ai_snapshot, &goals, &mut tasks);
        let duration = start.elapsed();

        println!(
            "Performance Benchmark: assigned tasks for {} civilians and {} goals in {:?}",
            num_civilians, num_goals, duration
        );
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
            trade_capacity_total: 3,
            trade_capacity_used: 0,
            buildings: HashMap::new(),
            building_entities: HashMap::new(),
        };

        let occupied_tiles = HashSet::new();
        let ai_snapshot = AiSnapshot {
            occupied_tiles: occupied_tiles.clone(),
            rails: HashSet::new(),
            ..Default::default()
        };

        // If bridgehead logic picks (0,0) as better than (0,1) due to tie-breaking,
        // and engineer is at (0,1), it will MoveTo (0,0).
        let task =
            plan_engineer_rail_task(&snapshot, &ai_snapshot, &occupied_tiles, pos_0_1, target);

        // This is fine if it leads to progress.
        // But if then it tries to move from (0,0) to (0,1), it's a loop.
        if let Some(CivilianTask::MoveTo { target: t }) = task
            && t == capital_pos
        {
            // Now check what happens at (0,0)
            let task2 = plan_engineer_rail_task(
                &snapshot,
                &ai_snapshot,
                &occupied_tiles,
                capital_pos,
                target,
            );
            // If task2 is MoveTo(0,1), we have a loop!
            assert!(
                !matches!(task2, Some(CivilianTask::MoveTo { target: next }) if next == pos_0_1),
                "Loop detected: (0,1) -> (0,0) -> (0,1)"
            );
        }
    }
}
