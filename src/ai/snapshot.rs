//! Unified game state snapshot for AI decision-making.
//!
//! This module captures all relevant game state once per turn, providing
//! a consistent view for both nation-level and civilian-level decisions.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};
use std::collections::{HashMap, HashSet};

use crate::ai::markers::AiNation;
use crate::civilians::types::{Civilian, CivilianKind, ProspectingKnowledge};
use crate::economy::goods::Good;
use crate::economy::market::{MARKET_RESOURCES, MarketPriceModel, MarketVolume};
use crate::economy::nation::{Capital, Nation};
use crate::economy::stockpile::{Stockpile, StockpileEntry};
use crate::economy::transport::{Depot, Rails};
use crate::economy::treasury::Treasury;
use crate::map::prospecting::PotentialMineral;
use crate::map::province::Province;
use crate::map::tile_pos::{HexExt, TilePosExt};
use crate::resources::{DevelopmentLevel, TileResource};
use crate::turn_system::TurnCounter;

/// Complete game state snapshot built once per turn.
#[derive(Resource, Default, Debug)]
pub struct AiSnapshot {
    pub turn: u32,
    pub nations: HashMap<Entity, NationSnapshot>,
    pub market: MarketSnapshot,
    /// All tiles currently occupied by any civilian (friendly or enemy)
    pub occupied_tiles: std::collections::HashSet<TilePos>,
    pub rails: std::collections::HashSet<(TilePos, TilePos)>,
}

impl AiSnapshot {
    pub fn get_nation(&self, entity: Entity) -> Option<&NationSnapshot> {
        self.nations.get(&entity)
    }
}

/// Snapshot of a single AI nation's state.
#[derive(Debug, Clone)]
pub struct NationSnapshot {
    pub entity: Entity,
    pub capital_pos: TilePos,
    pub treasury: i64,
    pub stockpile: HashMap<Good, StockpileEntry>,
    pub civilians: Vec<CivilianSnapshot>,
    pub connected_tiles: HashSet<TilePos>,
    pub unconnected_depots: Vec<DepotInfo>,
    /// Optimal depot locations calculated via greedy set-cover algorithm.
    pub suggested_depots: Vec<SuggestedDepot>,
    pub improvable_tiles: Vec<ImprovableTile>,
    pub owned_tiles: HashSet<TilePos>,
    pub depot_positions: HashSet<TilePos>,
    /// Tiles with potential minerals that haven't been prospected by this nation.
    pub prospectable_tiles: Vec<ProspectableTile>,
    /// Terrain type for each tile position (for build validation).
    pub tile_terrain: HashMap<TilePos, crate::map::tiles::TerrainType>,
    /// Technologies owned by this nation.
    pub technologies: crate::economy::technology::Technologies,
    /// Rails currently under construction by this nation.
    pub rail_constructions: Vec<RailConstructionSnapshot>,
    /// Trade capacity information.
    pub trade_capacity_total: u32,
    pub trade_capacity_used: u32,
    /// Buildings owned by this nation.
    pub buildings:
        HashMap<crate::economy::buildings::BuildingKind, crate::economy::buildings::Building>,
}

/// Snapshot of rail construction.
#[derive(Debug, Clone)]
pub struct RailConstructionSnapshot {
    pub from: TilePos,
    pub to: TilePos,
}

impl NationSnapshot {
    pub fn stockpile_amount(&self, good: Good) -> u32 {
        self.stockpile.get(&good).map(|e| e.total).unwrap_or(0)
    }

    pub fn available_amount(&self, good: Good) -> u32 {
        self.stockpile.get(&good).map(|e| e.available).unwrap_or(0)
    }

    /// Count civilians of a specific kind.
    pub fn civilian_count(&self, kind: CivilianKind) -> usize {
        self.civilians.iter().filter(|c| c.kind == kind).count()
    }

    /// Get civilians that haven't acted this turn.
    pub fn available_civilians(&self) -> impl Iterator<Item = &CivilianSnapshot> {
        self.civilians.iter().filter(|c| !c.has_moved)
    }

    /// Get available trade capacity (not currently used).
    pub fn trade_capacity_available(&self) -> u32 {
        self.trade_capacity_total
            .saturating_sub(self.trade_capacity_used)
    }

    /// Get trade capacity utilization as a percentage (0.0 to 1.0).
    pub fn trade_capacity_utilization(&self) -> f32 {
        if self.trade_capacity_total == 0 {
            return 0.0;
        }
        self.trade_capacity_used as f32 / self.trade_capacity_total as f32
    }
}

/// Snapshot of a civilian unit.
#[derive(Debug, Clone)]
pub struct CivilianSnapshot {
    pub entity: Entity,
    pub kind: CivilianKind,
    pub position: TilePos,
    pub has_moved: bool,
}

/// Info about a depot that needs rail connection.
#[derive(Debug, Clone)]
pub struct DepotInfo {
    pub position: TilePos,
    pub distance_from_capital: u32,
}

/// A suggested depot location with coverage information.
#[derive(Debug, Clone)]
pub struct SuggestedDepot {
    pub position: TilePos,
    pub covers_count: u32,
    pub distance_from_capital: u32,
}

/// Get all tiles covered by a depot at the given position (center + 6 neighbors).
pub fn depot_coverage(position: TilePos) -> impl Iterator<Item = TilePos> {
    let hex = position.to_hex();
    hex.all_neighbors()
        .into_iter()
        .filter_map(|h| h.to_tile_pos())
        .chain(std::iter::once(position))
}

/// Calculate optimal depot locations using a greedy set-cover algorithm.
///
/// The algorithm iteratively picks the owned tile that covers the most uncovered
/// resources until all resources are covered.
fn calculate_suggested_depots(
    resource_tiles: &HashSet<TilePos>,
    owned_tiles: &HashSet<TilePos>,
    depot_positions: &HashSet<TilePos>,
    capital_pos: TilePos,
    tile_terrain: &HashMap<TilePos, crate::map::tiles::TerrainType>,
) -> Vec<SuggestedDepot> {
    let capital_hex = capital_pos.to_hex();

    // Calculate which resources are already covered by existing depots and capital
    let mut covered_tiles: HashSet<TilePos> = HashSet::new();

    // Capital acts as a depot - covers itself + neighbors
    covered_tiles.extend(depot_coverage(capital_pos));

    // Each existing depot covers 7 tiles
    for &depot_pos in depot_positions {
        covered_tiles.extend(depot_coverage(depot_pos));
    }

    // Find uncovered resources
    let mut remaining: HashSet<TilePos> =
        resource_tiles.difference(&covered_tiles).copied().collect();

    let mut suggestions = Vec::new();

    // Greedy algorithm: pick the tile that covers the most uncovered resources
    while !remaining.is_empty() {
        let best = owned_tiles
            .iter()
            .filter(|pos| !depot_positions.contains(pos)) // No depot already here
            .filter(|pos| {
                // Filter out tiles with invalid terrain
                tile_terrain
                    .get(pos)
                    .map(crate::economy::transport::can_build_depot_on_terrain)
                    .unwrap_or(false)
            })
            .map(|&pos| {
                let coverage: HashSet<TilePos> = depot_coverage(pos).collect();
                let covers_count = remaining.intersection(&coverage).count() as u32;
                let distance = capital_hex.distance_to(pos.to_hex()) as u32;
                (pos, covers_count, distance)
            })
            .filter(|(_, count, _)| *count > 0) // Must cover at least 1 resource
            .max_by_key(|(_, count, dist)| (*count * 100, u32::MAX - dist)); // Prefer more coverage, then closer

        if let Some((pos, covers_count, distance)) = best {
            // Mark covered tiles as handled
            for covered in depot_coverage(pos) {
                remaining.remove(&covered);
            }
            suggestions.push(SuggestedDepot {
                position: pos,
                covers_count,
                distance_from_capital: distance,
            });
        } else {
            break; // No more valid positions
        }
    }

    // Sort by distance (closest first, with coverage as tiebreaker)
    suggestions.sort_by_key(|s| (s.distance_from_capital, u32::MAX - s.covers_count));

    suggestions
}

/// A tile that can be improved by a civilian.
#[derive(Debug, Clone)]
pub struct ImprovableTile {
    pub position: TilePos,
    pub resource_type: crate::resources::ResourceType,
    pub development: DevelopmentLevel,
    pub improver_kind: CivilianKind,
    pub distance_from_capital: u32,
}

/// A tile with potential minerals that can be prospected.
#[derive(Debug, Clone)]
pub struct ProspectableTile {
    pub position: TilePos,
    pub distance_from_capital: u32,
}

/// Snapshot of market state.
#[derive(Debug, Clone, Default)]
pub struct MarketSnapshot {
    pub prices: HashMap<Good, u32>,
}

impl MarketSnapshot {
    pub fn price_for(&self, good: Good) -> u32 {
        self.prices.get(&good).copied().unwrap_or(100)
    }
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

/// Builds the complete AI snapshot at the start of EnemyTurn.
#[allow(clippy::too_many_arguments)]
pub fn build_ai_snapshot(
    mut snapshot: ResMut<AiSnapshot>,
    turn: Res<TurnCounter>,
    pricing: Res<MarketPriceModel>,
    rails: Res<Rails>,
    trade_capacity: Res<crate::economy::trade_capacity::TradeCapacity>,
    ai_nations: Query<
        (
            Entity,
            &Capital,
            &Stockpile,
            &Treasury,
            &crate::economy::technology::Technologies,
            &crate::economy::buildings::Buildings,
        ),
        (With<AiNation>, With<Nation>),
    >,
    civilians: Query<(Entity, &Civilian)>,
    civilian_jobs: Query<&crate::civilians::types::CivilianJob>,
    rail_constructions: Query<&crate::economy::transport::RailConstruction>,
    depots: Query<&Depot>,
    provinces: Query<&Province>,
    tile_storage: Query<&TileStorage>,
    tile_resources: Query<&TileResource>,
    tile_terrain: Query<&crate::map::tiles::TerrainType>,
    potential_minerals: Query<&PotentialMineral>,
    prospecting: Option<Res<ProspectingKnowledge>>,
) {
    snapshot.turn = turn.current;

    // Collect all occupied tiles
    snapshot.occupied_tiles.clear();
    for (_, civilian) in civilians.iter() {
        snapshot.occupied_tiles.insert(civilian.position);
    }

    // Capture rail network
    snapshot.rails = rails.0.clone();

    snapshot.nations.clear();
    // Build market snapshot
    snapshot.market.prices.clear();
    for &good in MARKET_RESOURCES {
        let price = pricing.price_for(good, MarketVolume::default());
        snapshot.market.prices.insert(good, price);
    }

    let Ok(storage) = tile_storage.single() else {
        return;
    };

    // Build per-nation snapshots
    for (entity, capital, stockpile, treasury, technologies, buildings) in ai_nations.iter() {
        let capital_pos = capital.0;
        let capital_hex = capital_pos.to_hex();

        // Collect stockpile entries
        let stockpile_map: HashMap<Good, StockpileEntry> = stockpile
            .entries()
            .map(|entry| (entry.good, entry))
            .collect();

        // Find owned tiles from provinces
        let mut owned_tiles = HashSet::new();
        for province in provinces.iter() {
            if province.owner == Some(entity) {
                owned_tiles.extend(province.tiles.iter().copied());
            }
        }

        // Compute connected tiles via BFS from capital along rails
        let connected_tiles = compute_connected_tiles(capital_pos, entity, &owned_tiles, &rails);

        // Collect all depot positions for this nation
        let depot_positions: HashSet<TilePos> = depots
            .iter()
            .filter(|d| d.owner == entity)
            .map(|d| d.position)
            .collect();

        // Find unconnected depots
        let mut unconnected_depots: Vec<DepotInfo> = depots
            .iter()
            .filter(|d| d.owner == entity && !d.connected)
            .map(|d| {
                let hex = d.position.to_hex();
                DepotInfo {
                    position: d.position,
                    distance_from_capital: capital_hex.distance_to(hex) as u32,
                }
            })
            .collect();
        unconnected_depots.sort_by_key(|d| d.distance_from_capital);

        // Find resource tiles and improvable tiles
        let mut resource_tiles = HashSet::new();
        let mut improvable_tiles = Vec::new();
        for &tile_pos in &owned_tiles {
            let Some(tile_entity) = storage.get(&tile_pos) else {
                continue;
            };
            let Ok(resource) = tile_resources.get(tile_entity) else {
                continue;
            };
            if !resource.discovered {
                continue;
            }
            // Check prospecting knowledge for minerals
            let prospected = if resource.requires_prospecting() {
                if let Some(ref knowledge) = prospecting {
                    knowledge.is_discovered_by(tile_entity, entity)
                } else {
                    false
                }
            } else {
                true
            };
            if !prospected {
                continue;
            }
            // Track all discovered resource tiles for depot coverage calculation
            resource_tiles.insert(tile_pos);

            // Track improvable tiles (not at max development)
            if resource.development < DevelopmentLevel::Lv3
                && let Some(improver_kind) = improver_for_resource(&resource.resource_type)
            {
                let distance = capital_hex.distance_to(tile_pos.to_hex()) as u32;
                improvable_tiles.push(ImprovableTile {
                    position: tile_pos,
                    resource_type: resource.resource_type,
                    development: resource.development,
                    improver_kind,
                    distance_from_capital: distance,
                });
            }
        }
        improvable_tiles.sort_by_key(|t| (t.distance_from_capital, t.development as u8));

        // Find prospectable tiles (owned tiles with PotentialMineral not yet prospected by this nation)
        let mut prospectable_tiles = Vec::new();
        for &tile_pos in &owned_tiles {
            let Some(tile_entity) = storage.get(&tile_pos) else {
                continue;
            };
            // Check if tile has potential minerals
            if potential_minerals.get(tile_entity).is_ok() {
                // Check if we've already prospected this tile
                let already_prospected = prospecting
                    .as_ref()
                    .is_some_and(|k| k.is_discovered_by(tile_entity, entity));
                if !already_prospected {
                    let distance = capital_hex.distance_to(tile_pos.to_hex()) as u32;
                    prospectable_tiles.push(ProspectableTile {
                        position: tile_pos,
                        distance_from_capital: distance,
                    });
                }
            }
        }
        prospectable_tiles.sort_by_key(|t| t.distance_from_capital);

        // Collect terrain information for owned tiles
        let mut tile_terrain_map = HashMap::new();
        for &tile_pos in &owned_tiles {
            if let Some(tile_entity) = storage.get(&tile_pos)
                && let Ok(terrain) = tile_terrain.get(tile_entity)
            {
                tile_terrain_map.insert(tile_pos, *terrain);
            }
        }

        // Calculate optimal depot locations using greedy set-cover algorithm
        let suggested_depots = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain_map,
        );

        // Collect rail constructions for this nation
        let nation_rail_constructions: Vec<RailConstructionSnapshot> = rail_constructions
            .iter()
            .filter(|rc| rc.owner == entity)
            .map(|rc| RailConstructionSnapshot {
                from: rc.from,
                to: rc.to,
            })
            .collect();

        // Collect civilians for this nation (exclude those with active jobs)
        let nation_civilians: Vec<CivilianSnapshot> = civilians
            .iter()
            .filter(|(_, c)| c.owner == entity)
            .filter_map(|(e, c)| {
                // Exclude civilians that have active jobs (turns_remaining > 0).
                // Jobs that just completed (turns_remaining == 0) are OK to include because
                // job progress has already been applied for this turn, so these civilians are
                // now idle and available for new AI decisions.
                let has_active_job = civilian_jobs
                    .get(e)
                    .map(|job| job.turns_remaining > 0)
                    .unwrap_or(false);

                if has_active_job {
                    None
                } else {
                    Some(CivilianSnapshot {
                        entity: e,
                        kind: c.kind,
                        position: c.position,
                        has_moved: c.has_moved,
                    })
                }
            })
            .collect();

        // Get trade capacity
        let capacity_snapshot = trade_capacity.snapshot(entity);

        snapshot.nations.insert(
            entity,
            NationSnapshot {
                entity,
                capital_pos,
                treasury: treasury.available(),
                stockpile: stockpile_map,
                civilians: nation_civilians,
                connected_tiles,
                unconnected_depots,
                suggested_depots,
                improvable_tiles,
                owned_tiles,
                depot_positions,
                prospectable_tiles,
                tile_terrain: tile_terrain_map,
                technologies: technologies.clone(),
                rail_constructions: nation_rail_constructions,
                trade_capacity_total: capacity_snapshot.total,
                trade_capacity_used: capacity_snapshot.used,
                buildings: buildings.buildings.clone(),
            },
        );
    }
}

/// Compute tiles connected to capital via rails.
fn compute_connected_tiles(
    capital: TilePos,
    _owner: Entity,
    owned_tiles: &HashSet<TilePos>,
    rails: &Rails,
) -> HashSet<TilePos> {
    use std::collections::VecDeque;

    let mut connected = HashSet::new();
    let mut queue = VecDeque::new();

    connected.insert(capital);
    queue.push_back(capital);

    while let Some(current) = queue.pop_front() {
        for neighbor_hex in current.to_hex().all_neighbors() {
            let Some(neighbor_pos) = neighbor_hex.to_tile_pos() else {
                continue;
            };

            // Must be owned
            if !owned_tiles.contains(&neighbor_pos) {
                continue;
            }

            // Must have rail connection
            let edge = crate::economy::transport::ordered_edge(current, neighbor_pos);
            if !rails.0.contains(&edge) {
                continue;
            }

            if connected.insert(neighbor_pos) {
                queue.push_back(neighbor_pos);
            }
        }
    }

    connected
}

/// Determine which civilian kind can improve a resource type.
fn improver_for_resource(resource_type: &crate::resources::ResourceType) -> Option<CivilianKind> {
    use crate::resources::ResourceType;
    match resource_type {
        ResourceType::Grain | ResourceType::Cotton | ResourceType::Fruit => {
            Some(CivilianKind::Farmer)
        }
        ResourceType::Wool | ResourceType::Livestock => Some(CivilianKind::Rancher),
        ResourceType::Timber => Some(CivilianKind::Forester),
        ResourceType::Coal | ResourceType::Iron | ResourceType::Gold | ResourceType::Gems => {
            Some(CivilianKind::Miner)
        }
        ResourceType::Oil => Some(CivilianKind::Driller),
        ResourceType::Fish => None, // Ports, not civilians
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_target_days() {
        assert_eq!(resource_target_days(Good::Grain), 12.0);
        assert_eq!(resource_target_days(Good::Fish), 12.0);
        assert_eq!(resource_target_days(Good::Coal), 20.0);
        assert_eq!(resource_target_days(Good::Steel), 20.0);
    }

    #[test]
    fn depot_coverage_returns_seven_tiles() {
        let pos = TilePos::new(5, 5);
        let coverage: Vec<_> = depot_coverage(pos).collect();
        // Should include center + up to 6 neighbors (some may be filtered by to_tile_pos)
        assert!(
            coverage.contains(&pos),
            "coverage should include center tile"
        );
        assert!(coverage.len() <= 7, "coverage should be at most 7 tiles");
    }

    #[test]
    fn adjacent_resources_get_single_depot_suggestion() {
        // Get adjacent positions using hex neighbors
        let center = TilePos::new(10, 10);
        let center_hex = center.to_hex();
        let neighbors: Vec<TilePos> = center_hex
            .all_neighbors()
            .into_iter()
            .filter_map(|h| h.to_tile_pos())
            .take(2)
            .collect();

        // Create resource tiles: center + 2 adjacent tiles
        let mut resource_tiles: HashSet<TilePos> = HashSet::new();
        resource_tiles.insert(center);
        for n in &neighbors {
            resource_tiles.insert(*n);
        }

        let owned_tiles = resource_tiles.clone();
        let depot_positions = HashSet::new();
        let capital_pos = TilePos::new(50, 50); // Far away capital

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let suggestions = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // Should suggest only ONE depot that covers all adjacent resources
        assert_eq!(
            suggestions.len(),
            1,
            "adjacent resources should be covered by single depot"
        );
        assert!(
            suggestions[0].covers_count >= 3,
            "depot should cover all 3 resources"
        );
    }

    #[test]
    fn capital_coverage_excludes_nearby_resources() {
        let capital_pos = TilePos::new(10, 10);

        // Get an adjacent resource (should be covered by capital)
        let adjacent_resource = capital_pos
            .to_hex()
            .all_neighbors()
            .into_iter()
            .filter_map(|h| h.to_tile_pos())
            .next()
            .unwrap();

        // Resource far from capital (not covered)
        let far_resource = TilePos::new(30, 30);

        let resource_tiles: HashSet<_> = [adjacent_resource, far_resource].into_iter().collect();
        let mut owned_tiles = resource_tiles.clone();
        owned_tiles.insert(capital_pos);
        // Add capital coverage area to owned tiles
        for covered in depot_coverage(capital_pos) {
            owned_tiles.insert(covered);
        }

        let depot_positions = HashSet::new();

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let suggestions = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // Adjacent resource is covered by capital, so only far_resource needs a depot
        assert_eq!(
            suggestions.len(),
            1,
            "only far resource should need a depot"
        );
        // The suggestion should be for the far resource area
        let suggestion_hex = suggestions[0].position.to_hex();
        let far_hex = far_resource.to_hex();
        assert!(
            suggestion_hex.distance_to(far_hex) <= 1,
            "suggested depot should cover far resource"
        );
    }

    #[test]
    fn existing_depot_prevents_duplicate_suggestion() {
        let resource_pos = TilePos::new(10, 10);
        let resource_tiles: HashSet<_> = [resource_pos].into_iter().collect();
        let owned_tiles = resource_tiles.clone();

        // Depot already exists at this resource position
        let depot_positions: HashSet<_> = [resource_pos].into_iter().collect();
        let capital_pos = TilePos::new(50, 50);

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let suggestions = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // No suggestions needed - existing depot covers the resource
        assert!(
            suggestions.is_empty(),
            "no suggestions needed when depot already covers resources"
        );
    }

    #[test]
    fn distant_resource_clusters_get_separate_depots() {
        // Two clusters very far apart (can't be covered by single depot)
        let cluster1 = TilePos::new(5, 5);
        let cluster2 = TilePos::new(30, 30); // Very far away

        let resource_tiles: HashSet<_> = [cluster1, cluster2].into_iter().collect();
        let owned_tiles: HashSet<_> = [cluster1, cluster2].into_iter().collect();
        let depot_positions = HashSet::new();
        let capital_pos = TilePos::new(50, 50); // Far away capital

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let suggestions = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // Should suggest 2 depots (one for each cluster)
        assert_eq!(
            suggestions.len(),
            2,
            "distant clusters should each get their own depot"
        );
    }

    #[test]
    fn greedy_algorithm_optimizes_coverage() {
        // Create a cluster of 3 resources + 1 isolated resource
        let center = TilePos::new(10, 10);
        let center_hex = center.to_hex();
        let neighbors: Vec<TilePos> = center_hex
            .all_neighbors()
            .into_iter()
            .filter_map(|h| h.to_tile_pos())
            .take(2)
            .collect();

        // Cluster: center + 2 neighbors (3 resources)
        // Isolated: far away (1 resource)
        let isolated = TilePos::new(30, 30);

        let mut resources: HashSet<TilePos> = HashSet::new();
        resources.insert(center);
        for n in &neighbors {
            resources.insert(*n);
        }
        resources.insert(isolated);

        let owned_tiles = resources.clone();
        let depot_positions = HashSet::new();
        let capital_pos = TilePos::new(50, 50);

        // Create terrain map with buildable terrain (Grass)
        let mut tile_terrain = HashMap::new();
        for &pos in &owned_tiles {
            tile_terrain.insert(pos, crate::map::tiles::TerrainType::Grass);
        }

        let suggestions = calculate_suggested_depots(
            &resources,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // Greedy should pick efficiently: 2 depots for 4 resources
        // (one covering cluster of 3, one for isolated)
        assert_eq!(
            suggestions.len(),
            2,
            "should suggest 2 depots for 4 resources (cluster + isolated)"
        );
    }

    #[test]
    fn depot_suggestions_filter_out_invalid_terrain() {
        // Resource on grass (can build depot)
        let resource_grass = TilePos::new(10, 10);
        // Resource on mountain (cannot build depot)
        let resource_mountain = TilePos::new(15, 15);
        // Resource on water (cannot build depot)
        let resource_water = TilePos::new(20, 20);

        let resource_tiles: HashSet<_> = [resource_grass, resource_mountain, resource_water]
            .into_iter()
            .collect();
        let owned_tiles = resource_tiles.clone();
        let depot_positions = HashSet::new();
        let capital_pos = TilePos::new(50, 50); // Far away capital

        // Create terrain map with different terrain types
        let mut tile_terrain = HashMap::new();
        tile_terrain.insert(resource_grass, crate::map::tiles::TerrainType::Grass);
        tile_terrain.insert(resource_mountain, crate::map::tiles::TerrainType::Mountain);
        tile_terrain.insert(resource_water, crate::map::tiles::TerrainType::Water);

        let suggestions = calculate_suggested_depots(
            &resource_tiles,
            &owned_tiles,
            &depot_positions,
            capital_pos,
            &tile_terrain,
        );

        // Only the grass tile should get a depot suggestion
        assert_eq!(
            suggestions.len(),
            1,
            "only grass tile should get depot suggestion"
        );
        assert_eq!(
            suggestions[0].position, resource_grass,
            "suggested depot should be on grass terrain"
        );
    }

    #[test]
    fn civilians_with_active_jobs_excluded_from_available() {
        use std::collections::HashMap;

        // Create placeholder entities for testing
        let entity1 = Entity::PLACEHOLDER;
        let entity2 = Entity::PLACEHOLDER;
        let entity3 = Entity::PLACEHOLDER;

        let snapshot = NationSnapshot {
            entity: Entity::PLACEHOLDER,
            capital_pos: TilePos::new(0, 0),
            treasury: 1000,
            stockpile: HashMap::new(),
            civilians: vec![
                CivilianSnapshot {
                    entity: entity1,
                    kind: CivilianKind::Engineer,
                    position: TilePos::new(5, 5),
                    has_moved: false,
                },
                CivilianSnapshot {
                    entity: entity2,
                    kind: CivilianKind::Engineer,
                    position: TilePos::new(6, 6),
                    has_moved: true, // Moved this turn
                },
                CivilianSnapshot {
                    entity: entity3,
                    kind: CivilianKind::Prospector,
                    position: TilePos::new(7, 7),
                    has_moved: false,
                },
            ],
            connected_tiles: HashSet::new(),
            unconnected_depots: vec![],
            suggested_depots: vec![],
            improvable_tiles: vec![],
            owned_tiles: HashSet::new(),
            depot_positions: HashSet::new(),
            prospectable_tiles: vec![],
            tile_terrain: HashMap::new(),
            technologies: crate::economy::technology::Technologies::new(),
            rail_constructions: vec![],
            trade_capacity_total: 3,
            trade_capacity_used: 0,
            buildings: HashMap::new(),
        };

        // Only civilians with has_moved = false should be available
        // Civilian 1: has_moved = false (available)
        // Civilian 2: has_moved = true (not available)
        // Civilian 3: has_moved = false (available)
        let available: Vec<_> = snapshot.available_civilians().collect();
        assert_eq!(available.len(), 2, "only 2 civilians should be available");
    }
}
