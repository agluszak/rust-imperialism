use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::iter;

use crate::{
    civilians::types::ProspectingKnowledge,
    economy::transport::{Depot, Port},
    map::tile_pos::{HexExt, TilePosExt},
    resources::{ResourceType, TileResource},
};
use bevy_ecs_tilemap::prelude::{TilePos, TileStorage};

use super::workforce::Workforce;
use super::{goods::Good, stockpile::Stockpile};
use crate::turn_system::{TurnPhase, TurnSystem};

/// Resource that stores the total connected production output for each nation.
/// `(u32, u32)` is `(number_of_improvements, total_output)`
#[derive(Resource, Default, Debug)]
pub struct ConnectedProduction(pub HashMap<Entity, HashMap<ResourceType, (u32, u32)>>);

/// Calculates the total production from all resource tiles connected to the rail network.
/// This system runs after `compute_rail_connectivity`.
pub fn calculate_connected_production(
    mut production: ResMut<ConnectedProduction>,
    connected_depots: Query<&Depot>,
    connected_ports: Query<&Port>,
    tile_storage: Query<&TileStorage>,
    tile_resources: Query<&TileResource>,
    prospecting_knowledge: Res<ProspectingKnowledge>,
) {
    // Clear previous turn's data
    production.0.clear();

    let mut processed_tiles: HashSet<TilePos> = HashSet::new();

    let Ok(tile_storage) = tile_storage.single() else {
        return;
    };

    let mut process_improvement = |owner: Entity, position: TilePos| {
        let center_hex = position.to_hex();
        let neighbors = center_hex.all_neighbors();
        let tiles_to_check = neighbors.iter().copied().chain(iter::once(center_hex));

        for hex in tiles_to_check {
            if let Some(tile_pos) = hex.to_tile_pos() {
                if processed_tiles.contains(&tile_pos) {
                    continue; // Avoid double-counting
                }
                if let Some(tile_entity) = tile_storage.get(&tile_pos)
                    && let Ok(resource) = tile_resources.get(tile_entity)
                    && resource.discovered
                    && resource.get_output() > 0
                {
                    if resource.requires_prospecting()
                        && !prospecting_knowledge.is_discovered_by(tile_entity, owner)
                    {
                        continue;
                    }
                    let nation_production = production.0.entry(owner).or_default();
                    let entry = nation_production.entry(resource.resource_type).or_default();
                    entry.0 += 1; // Increment improvement count
                    entry.1 += resource.get_output(); // Add production output
                    processed_tiles.insert(tile_pos);
                }
            }
        }
    };

    // Process connected depots
    for depot in connected_depots.iter().filter(|d| d.connected) {
        process_improvement(depot.owner, depot.position);
    }

    // Process connected ports
    for port in connected_ports.iter().filter(|p| p.connected) {
        process_improvement(port.owner, port.position);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildingKind {
    // Production buildings
    TextileMill,          // 2×Cotton OR 2×Wool → 1×Fabric
    LumberMill,           // 2×Timber → 1×Lumber OR 1×Paper
    SteelMill,            // 1×Iron + 1×Coal → 1×Steel
    FoodProcessingCenter, // 2×Grain + 1×Fruit + 1×(Livestock|Fish) → 2×CannedFood
    ClothingFactory,      // 2×Fabric → 1×Clothing
    FurnitureFactory,     // 2×Lumber → 1×Furniture
    MetalWorks,           // 2×Steel → 1×Hardware OR 1×Armaments
    Refinery,             // 2×Oil → 1×Fuel
    Railyard,             // 1×Steel + 1×Lumber → 1×Transport

    // Worker-related buildings (no production capacity)
    Capitol,     // Recruit untrained workers
    TradeSchool, // Train workers
    PowerPlant,  // Convert fuel to labor
}

/// What input material a building should use for production
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionChoice {
    // For TextileMill: choose between Cotton or Wool
    UseCotton,
    UseWool,

    // For LumberMill: choose between Lumber or Paper
    MakeLumber,
    MakePaper,

    // For FoodProcessingCenter: choose between Livestock or Fish
    UseLivestock,
    UseFish,

    // For MetalWorks: choose between Hardware or Armaments
    MakeHardware,
    MakeArmaments,
}

/// Production settings for a building (persists turn-to-turn)
#[derive(Component, Debug, Clone)]
pub struct ProductionSettings {
    /// What input material to use (e.g., Cotton vs Wool for textile mill)
    pub choice: ProductionChoice,
    /// How many units to produce this turn (capped by capacity and inputs)
    pub target_output: u32,
}

impl Default for ProductionSettings {
    fn default() -> Self {
        Self {
            choice: ProductionChoice::UseCotton,
            target_output: 0,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Building {
    pub kind: BuildingKind,
    pub capacity: u32, // Maximum output per turn
}

impl Building {
    pub fn textile_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::TextileMill,
            capacity,
        }
    }

    pub fn lumber_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::LumberMill,
            capacity,
        }
    }

    pub fn steel_mill(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::SteelMill,
            capacity,
        }
    }

    pub fn food_processing_center(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::FoodProcessingCenter,
            capacity,
        }
    }

    pub fn clothing_factory(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::ClothingFactory,
            capacity,
        }
    }

    pub fn furniture_factory(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::FurnitureFactory,
            capacity,
        }
    }

    pub fn metal_works(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::MetalWorks,
            capacity,
        }
    }

    pub fn refinery(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::Refinery,
            capacity,
        }
    }

    pub fn railyard() -> Self {
        Self {
            kind: BuildingKind::Railyard,
            capacity: u32::MAX, // Unlimited capacity - limited only by inputs and labor
        }
    }

    pub fn capitol() -> Self {
        Self {
            kind: BuildingKind::Capitol,
            capacity: 0, // Not a production building
        }
    }

    pub fn trade_school() -> Self {
        Self {
            kind: BuildingKind::TradeSchool,
            capacity: 0, // Not a production building
        }
    }

    pub fn power_plant(capacity: u32) -> Self {
        Self {
            kind: BuildingKind::PowerPlant,
            capacity, // Fuel → labor conversion capacity
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ingredient {
    pub good: Good,
    pub amount: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductAmount {
    pub good: Good,
    pub amount: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipeVariant {
    inputs: &'static [Ingredient],
    outputs: &'static [ProductAmount],
}

impl RecipeVariant {
    pub fn inputs(&self) -> &'static [Ingredient] {
        self.inputs
    }

    pub fn outputs(&self) -> &'static [ProductAmount] {
        self.outputs
    }

    pub fn primary_output(&self) -> Option<ProductAmount> {
        self.outputs.first().copied()
    }

    pub fn primary_output_good(&self) -> Option<Good> {
        self.primary_output().map(|output| output.good)
    }

    pub fn primary_output_amount(&self) -> u32 {
        self.primary_output()
            .map(|output| output.amount)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipeVariantInfo {
    pub choice: Option<ProductionChoice>,
    pub variant: RecipeVariant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductionRecipe {
    variants: &'static [RecipeVariantDefinition],
}

impl ProductionRecipe {
    pub fn variant_for_choice(&self, choice: ProductionChoice) -> Option<RecipeVariant> {
        self.variants
            .iter()
            .find(|definition| definition.choice == Some(choice))
            .or_else(|| {
                self.variants
                    .iter()
                    .find(|definition| definition.choice.is_none())
            })
            .map(|definition| definition.variant)
    }

    pub fn variants_for_output(&self, output_good: Good) -> Vec<RecipeVariantInfo> {
        self.variants_iter(output_good).collect()
    }

    pub fn input_amount_for(&self, output_good: Good, input_good: Good) -> Option<u32> {
        self.variants_iter(output_good).find_map(|info| {
            info.variant
                .inputs
                .iter()
                .find(|ingredient| ingredient.good == input_good)
                .map(|ingredient| ingredient.amount)
        })
    }

    pub fn produces(&self, output_good: Good) -> bool {
        self.variants_iter(output_good).next().is_some()
    }

    fn variants_iter(&self, output_good: Good) -> impl Iterator<Item = RecipeVariantInfo> + '_ {
        self.variants
            .iter()
            .filter(move |definition| {
                definition
                    .variant
                    .primary_output_good()
                    .is_some_and(|good| good == output_good)
            })
            .map(|definition| RecipeVariantInfo {
                choice: definition.choice,
                variant: definition.variant,
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecipeVariantDefinition {
    choice: Option<ProductionChoice>,
    variant: RecipeVariant,
}

const TEXTILE_COTTON_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Cotton,
    amount: 2,
}];
const TEXTILE_WOOL_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Wool,
    amount: 2,
}];
const TEXTILE_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Fabric,
    amount: 1,
}];
const TEXTILE_VARIANTS: [RecipeVariantDefinition; 2] = [
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::UseCotton),
        variant: RecipeVariant {
            inputs: &TEXTILE_COTTON_INPUTS,
            outputs: &TEXTILE_OUTPUTS,
        },
    },
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::UseWool),
        variant: RecipeVariant {
            inputs: &TEXTILE_WOOL_INPUTS,
            outputs: &TEXTILE_OUTPUTS,
        },
    },
];
const TEXTILE_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &TEXTILE_VARIANTS,
};

const LUMBER_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Timber,
    amount: 2,
}];
const LUMBER_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Lumber,
    amount: 1,
}];
const PAPER_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Paper,
    amount: 1,
}];
const LUMBER_VARIANTS: [RecipeVariantDefinition; 2] = [
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::MakeLumber),
        variant: RecipeVariant {
            inputs: &LUMBER_INPUTS,
            outputs: &LUMBER_OUTPUTS,
        },
    },
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::MakePaper),
        variant: RecipeVariant {
            inputs: &LUMBER_INPUTS,
            outputs: &PAPER_OUTPUTS,
        },
    },
];
const LUMBER_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &LUMBER_VARIANTS,
};

const STEEL_INPUTS: [Ingredient; 2] = [
    Ingredient {
        good: Good::Iron,
        amount: 1,
    },
    Ingredient {
        good: Good::Coal,
        amount: 1,
    },
];
const STEEL_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Steel,
    amount: 1,
}];
const STEEL_VARIANTS: [RecipeVariantDefinition; 1] = [RecipeVariantDefinition {
    choice: None,
    variant: RecipeVariant {
        inputs: &STEEL_INPUTS,
        outputs: &STEEL_OUTPUTS,
    },
}];
const STEEL_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &STEEL_VARIANTS,
};

const FOOD_LIVESTOCK_INPUTS: [Ingredient; 3] = [
    Ingredient {
        good: Good::Grain,
        amount: 2,
    },
    Ingredient {
        good: Good::Fruit,
        amount: 1,
    },
    Ingredient {
        good: Good::Livestock,
        amount: 1,
    },
];
const FOOD_FISH_INPUTS: [Ingredient; 3] = [
    Ingredient {
        good: Good::Grain,
        amount: 2,
    },
    Ingredient {
        good: Good::Fruit,
        amount: 1,
    },
    Ingredient {
        good: Good::Fish,
        amount: 1,
    },
];
const FOOD_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::CannedFood,
    amount: 2,
}];
const FOOD_VARIANTS: [RecipeVariantDefinition; 2] = [
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::UseLivestock),
        variant: RecipeVariant {
            inputs: &FOOD_LIVESTOCK_INPUTS,
            outputs: &FOOD_OUTPUTS,
        },
    },
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::UseFish),
        variant: RecipeVariant {
            inputs: &FOOD_FISH_INPUTS,
            outputs: &FOOD_OUTPUTS,
        },
    },
];
const FOOD_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &FOOD_VARIANTS,
};

const CLOTHING_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Fabric,
    amount: 2,
}];
const CLOTHING_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Clothing,
    amount: 1,
}];
const CLOTHING_VARIANTS: [RecipeVariantDefinition; 1] = [RecipeVariantDefinition {
    choice: None,
    variant: RecipeVariant {
        inputs: &CLOTHING_INPUTS,
        outputs: &CLOTHING_OUTPUTS,
    },
}];
const CLOTHING_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &CLOTHING_VARIANTS,
};

const FURNITURE_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Lumber,
    amount: 2,
}];
const FURNITURE_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Furniture,
    amount: 1,
}];
const FURNITURE_VARIANTS: [RecipeVariantDefinition; 1] = [RecipeVariantDefinition {
    choice: None,
    variant: RecipeVariant {
        inputs: &FURNITURE_INPUTS,
        outputs: &FURNITURE_OUTPUTS,
    },
}];
const FURNITURE_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &FURNITURE_VARIANTS,
};

const METAL_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Steel,
    amount: 2,
}];
const HARDWARE_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Hardware,
    amount: 1,
}];
const ARMAMENT_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Armaments,
    amount: 1,
}];
const METAL_VARIANTS: [RecipeVariantDefinition; 2] = [
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::MakeHardware),
        variant: RecipeVariant {
            inputs: &METAL_INPUTS,
            outputs: &HARDWARE_OUTPUTS,
        },
    },
    RecipeVariantDefinition {
        choice: Some(ProductionChoice::MakeArmaments),
        variant: RecipeVariant {
            inputs: &METAL_INPUTS,
            outputs: &ARMAMENT_OUTPUTS,
        },
    },
];
const METAL_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &METAL_VARIANTS,
};

const REFINERY_INPUTS: [Ingredient; 1] = [Ingredient {
    good: Good::Oil,
    amount: 2,
}];
const REFINERY_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Fuel,
    amount: 1,
}];
const REFINERY_VARIANTS: [RecipeVariantDefinition; 1] = [RecipeVariantDefinition {
    choice: None,
    variant: RecipeVariant {
        inputs: &REFINERY_INPUTS,
        outputs: &REFINERY_OUTPUTS,
    },
}];
const REFINERY_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &REFINERY_VARIANTS,
};

const RAILYARD_INPUTS: [Ingredient; 2] = [
    Ingredient {
        good: Good::Steel,
        amount: 1,
    },
    Ingredient {
        good: Good::Lumber,
        amount: 1,
    },
];
const RAILYARD_OUTPUTS: [ProductAmount; 1] = [ProductAmount {
    good: Good::Transport,
    amount: 1,
}];
const RAILYARD_VARIANTS: [RecipeVariantDefinition; 1] = [RecipeVariantDefinition {
    choice: None,
    variant: RecipeVariant {
        inputs: &RAILYARD_INPUTS,
        outputs: &RAILYARD_OUTPUTS,
    },
}];
const RAILYARD_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &RAILYARD_VARIANTS,
};

const PRODUCTION_RECIPES: &[(BuildingKind, &ProductionRecipe)] = &[
    (BuildingKind::TextileMill, &TEXTILE_RECIPE),
    (BuildingKind::LumberMill, &LUMBER_RECIPE),
    (BuildingKind::SteelMill, &STEEL_RECIPE),
    (BuildingKind::FoodProcessingCenter, &FOOD_RECIPE),
    (BuildingKind::ClothingFactory, &CLOTHING_RECIPE),
    (BuildingKind::FurnitureFactory, &FURNITURE_RECIPE),
    (BuildingKind::MetalWorks, &METAL_RECIPE),
    (BuildingKind::Refinery, &REFINERY_RECIPE),
    (BuildingKind::Railyard, &RAILYARD_RECIPE),
];

pub fn production_recipe(kind: BuildingKind) -> Option<&'static ProductionRecipe> {
    PRODUCTION_RECIPES
        .iter()
        .find_map(|(recipe_kind, recipe)| (*recipe_kind == kind).then_some(*recipe))
}

pub fn building_for_output(output_good: Good) -> Option<BuildingKind> {
    PRODUCTION_RECIPES
        .iter()
        .find_map(|(kind, recipe)| recipe.produces(output_good).then_some(*kind))
}

pub fn input_requirement_per_unit(
    kind: BuildingKind,
    output_good: Good,
    input_good: Good,
) -> Option<u32> {
    production_recipe(kind)?.input_amount_for(output_good, input_good)
}

/// Collection of all buildings for a nation
#[derive(Component, Debug, Clone, Default)]
pub struct Buildings {
    pub buildings: HashMap<BuildingKind, Building>,
}

impl Buildings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_all_initial() -> Self {
        let mut buildings = HashMap::new();
        buildings.insert(BuildingKind::TextileMill, Building::textile_mill(8));
        buildings.insert(BuildingKind::LumberMill, Building::lumber_mill(4));
        buildings.insert(BuildingKind::SteelMill, Building::steel_mill(4));
        buildings.insert(
            BuildingKind::FoodProcessingCenter,
            Building::food_processing_center(4),
        );
        buildings.insert(BuildingKind::ClothingFactory, Building::clothing_factory(2));
        buildings.insert(
            BuildingKind::FurnitureFactory,
            Building::furniture_factory(2),
        );
        buildings.insert(BuildingKind::MetalWorks, Building::metal_works(2));
        buildings.insert(BuildingKind::Refinery, Building::refinery(2));
        buildings.insert(BuildingKind::Railyard, Building::railyard());
        Self { buildings }
    }

    pub fn get(&self, kind: BuildingKind) -> Option<Building> {
        self.buildings.get(&kind).copied()
    }

    pub fn insert(&mut self, building: Building) {
        self.buildings.insert(building.kind, building);
    }
}

/// Runs production across all entities that have both a Stockpile and a Building.
/// Consumes reserved resources and produces outputs.
/// Production rules follow 2:1 ratios (2 inputs → 1 output).
/// Production now requires labor points from workers.
pub fn run_production(
    turn: Res<TurnSystem>,
    mut q: Query<(
        Option<&Workforce>,
        &mut Stockpile,
        &Building,
        &mut ProductionSettings,
    )>,
) {
    if turn.phase != TurnPhase::Processing {
        return;
    }

    for (workforce_opt, mut stock, building, mut settings) in q.iter_mut() {
        // Calculate available labor (0 if no workforce)
        let available_labor = workforce_opt.map(|w| w.available_labor()).unwrap_or(0);

        // Each unit of production requires 1 labor point
        // This acts as another constraint on production alongside capacity and inputs
        let max_from_labor = available_labor;
        let Some(recipe) = production_recipe(building.kind) else {
            continue;
        };

        let desired_output = settings
            .target_output
            .min(max_from_labor)
            .min(building.capacity);
        if desired_output == 0 {
            settings.target_output = 0;
            continue;
        }

        let Some(variant) = recipe.variant_for_choice(settings.choice) else {
            settings.target_output = 0;
            debug!(
                "Skipping production for {:?}: no variant for choice {:?}",
                building.kind, settings.choice
            );
            continue;
        };

        let output_per_batch = variant.primary_output_amount();
        if output_per_batch == 0 {
            settings.target_output = 0;
            continue;
        }

        let target_batches = desired_output.div_ceil(output_per_batch);
        if target_batches == 0 {
            settings.target_output = 0;
            continue;
        }

        let (produced_output, consumption) = execute_variant(&mut stock, variant, target_batches);

        if produced_output < desired_output {
            log_production_shortfall(
                building.kind,
                variant,
                desired_output,
                produced_output,
                &consumption,
            );
        }

        settings.target_output = produced_output;
    }
}

#[derive(Clone, Debug)]
struct ConsumptionRecord {
    ingredient: Ingredient,
    consumed: u32,
    required: u32,
}

fn execute_variant(
    stock: &mut Stockpile,
    variant: RecipeVariant,
    target_batches: u32,
) -> (u32, Vec<ConsumptionRecord>) {
    if target_batches == 0 {
        return (0, Vec::new());
    }

    let mut actual_batches = target_batches;
    let mut consumption = Vec::with_capacity(variant.inputs().len());

    for ingredient in variant.inputs() {
        let required = ingredient.amount.saturating_mul(target_batches);
        let consumed = if required > 0 {
            stock.consume_reserved(ingredient.good, required)
        } else {
            0
        };
        if ingredient.amount > 0 {
            actual_batches = actual_batches.min(consumed / ingredient.amount);
        }
        consumption.push(ConsumptionRecord {
            ingredient: *ingredient,
            consumed,
            required,
        });
    }

    let primary_output = variant.primary_output();
    let mut produced_primary = 0;

    for output in variant.outputs() {
        let produced_amount = actual_batches.saturating_mul(output.amount);
        if produced_amount > 0 {
            stock.add(output.good, produced_amount);
        }
        if primary_output.is_some_and(|primary| primary.good == output.good) {
            produced_primary = produced_amount;
        }
    }

    (produced_primary, consumption)
}

fn log_production_shortfall(
    building_kind: BuildingKind,
    variant: RecipeVariant,
    requested_output: u32,
    produced_output: u32,
    consumption: &[ConsumptionRecord],
) {
    if produced_output >= requested_output {
        return;
    }

    let output_good = variant.primary_output_good().unwrap_or(Good::Fabric);

    let details = if consumption.is_empty() {
        "no inputs consumed".to_string()
    } else {
        consumption
            .iter()
            .map(|record| {
                format!(
                    "{:?} {}/{}",
                    record.ingredient.good, record.consumed, record.required
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    info!(
        "{:?}: requested {} {:?} but produced {} ({})",
        building_kind, requested_output, output_good, produced_output, details
    );
}
