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
    pub civilians_to_hire: Vec<CivilianKind>,
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
    generate_infrastructure_goals(nation, &mut plan.goals);
    generate_improvement_goals(nation, &mut plan.goals);
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
        // Priority decreases with distance
        let priority = (1.0 / (1.0 + depot.distance_from_capital as f32)).clamp(0.2, 0.9);
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

    // Second pass: Improvement specialists
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
    // If we're at the target, build the depot
    if engineer_pos == target {
        return Some(CivilianTask::BuildDepot);
    }

    // If we're on a connected tile, try to build rail toward target
    if nation.connected_tiles.contains(&engineer_pos) {
        // Find adjacent tile that gets us closer to target
        if let Some(next_tile) = find_step_toward(engineer_pos, target, &nation.owned_tiles)
            && is_adjacent(engineer_pos, next_tile)
        {
            return Some(CivilianTask::BuildRailTo { target: next_tile });
        }
    }

    // If not on connected tiles, move to the closest connected tile first
    // This ensures we build rails to the depot location (connected infrastructure)
    // Note: If connected_tiles is empty (no rail network yet), we fall through to the next case
    if !nation.connected_tiles.contains(&engineer_pos) {
        let closest_connected = nation
            .connected_tiles
            .iter()
            .min_by_key(|t| engineer_pos.to_hex().distance_to(t.to_hex()));
        if let Some(&connected_tile) = closest_connected {
            return Some(CivilianTask::MoveTo { target: connected_tile });
        }
    }

    // Fallback: if we're on connected tiles but can't build rail adjacent (e.g., blocked),
    // move directly toward target
    if nation.owned_tiles.contains(&target) {
        return Some(CivilianTask::MoveTo { target });
    }

    None
}

/// Plan an engineer task to build rail connecting to an existing depot.
fn plan_engineer_rail_task(
    nation: &NationSnapshot,
    engineer_pos: TilePos,
    depot_pos: TilePos,
) -> Option<CivilianTask> {
    // If we're on a connected tile, try to build rail toward the depot
    if nation.connected_tiles.contains(&engineer_pos) {
        // Find adjacent tile that gets us closer to depot
        if let Some(next_tile) = find_step_toward(engineer_pos, depot_pos, &nation.owned_tiles)
            && is_adjacent(engineer_pos, next_tile)
            && !nation.connected_tiles.contains(&next_tile)
        {
            return Some(CivilianTask::BuildRailTo { target: next_tile });
        }
    }

    // If not on connected tiles, move directly to the closest connected tile
    // Note: If connected_tiles is empty (no rail network yet), we fall through to the next case
    if !nation.connected_tiles.contains(&engineer_pos) {
        // Find the closest connected tile
        let closest_connected = nation
            .connected_tiles
            .iter()
            .min_by_key(|t| engineer_pos.to_hex().distance_to(t.to_hex()));
        if let Some(&connected_tile) = closest_connected {
            return Some(CivilianTask::MoveTo { target: connected_tile });
        }
    } else {
        // We're on connected tiles - move directly toward the depot
        if nation.owned_tiles.contains(&depot_pos) {
            return Some(CivilianTask::MoveTo { target: depot_pos });
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
        .min_by_key(|pos| pos.to_hex().distance_to(to_hex))
}

/// Check if two positions are adjacent on the hex grid.
fn is_adjacent(a: TilePos, b: TilePos) -> bool {
    a.to_hex().distance_to(b.to_hex()) == 1
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
        
        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            id: crate::economy::nation::NationId(1),
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
        
        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            id: crate::economy::nation::NationId(1),
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
        };
        
        let task = plan_engineer_depot_task(&snapshot, engineer_pos, target);
        
        // Should build rail to adjacent tile toward target
        assert!(matches!(task, Some(CivilianTask::BuildRailTo { target: t }) if t == next_step));
    }
}
