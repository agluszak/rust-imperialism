use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::goods::Good;
use crate::economy::stockpile::Stockpile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
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
    Shipyard,             // 1×Steel + 1×Lumber + 1×Fuel → 1×Ship

    // Worker-related buildings (no production capacity)
    Capitol,     // Recruit untrained workers
    TradeSchool, // Train workers
    PowerPlant,  // Convert fuel to labor
}

/// Production settings for a building (persists turn-to-turn)
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[derive(Default)]
pub struct ProductionSettings {
    /// How many units to produce this turn (capped by capacity and inputs)
    pub target_output: u32,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
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

    pub fn shipyard() -> Self {
        Self {
            kind: BuildingKind::Shipyard,
            capacity: u32::MAX,
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
    pub inputs: &'static [Ingredient],
    pub outputs: &'static [ProductAmount],
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
pub struct ProductionRecipe {
    pub variants: &'static [RecipeVariant],
}

impl ProductionRecipe {
    /// Select the best variant based on stockpile availability.
    /// For buildings with multiple input options (e.g., Cotton vs Wool),
    /// this chooses the variant with the most available inputs.
    /// Falls back to the first variant if none have clear preference.
    pub fn best_variant_for_stockpile(&self, stockpile: &Stockpile) -> Option<RecipeVariant> {
        if self.variants.is_empty() {
            return None;
        }

        // If there's only one variant, use it
        if self.variants.len() == 1 {
            return Some(self.variants[0]);
        }

        // Score each variant by available inputs
        let mut best_variant = self.variants[0];
        let mut best_score = score_variant_availability(&self.variants[0], stockpile);

        for variant in &self.variants[1..] {
            let score = score_variant_availability(variant, stockpile);
            if score > best_score {
                best_score = score;
                best_variant = *variant;
            }
        }

        Some(best_variant)
    }

    pub fn variants_for_output(&self, output_good: Good) -> Vec<RecipeVariant> {
        self.variants_iter(output_good).collect()
    }

    pub fn input_amount_for(&self, output_good: Good, input_good: Good) -> Option<u32> {
        self.variants_iter(output_good).find_map(|variant| {
            variant
                .inputs
                .iter()
                .find(|ingredient| ingredient.good == input_good)
                .map(|ingredient| ingredient.amount)
        })
    }

    pub fn produces(&self, output_good: Good) -> bool {
        self.variants_iter(output_good).next().is_some()
    }

    fn variants_iter(&self, output_good: Good) -> impl Iterator<Item = RecipeVariant> + '_ {
        self.variants
            .iter()
            .filter(move |variant| {
                variant
                    .primary_output_good()
                    .is_some_and(|good| good == output_good)
            })
            .copied()
    }
}

/// Score a variant based on how many batches could be produced with available inputs.
/// Higher score means more production is possible with this variant.
fn score_variant_availability(variant: &RecipeVariant, stockpile: &Stockpile) -> u32 {
    variant
        .inputs
        .iter()
        .map(|ingredient| {
            if ingredient.amount == 0 {
                u32::MAX
            } else {
                stockpile.get_available(ingredient.good) / ingredient.amount
            }
        })
        .min()
        .unwrap_or(u32::MAX)
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
const TEXTILE_VARIANTS: [RecipeVariant; 2] = [
    RecipeVariant {
        inputs: &TEXTILE_COTTON_INPUTS,
        outputs: &TEXTILE_OUTPUTS,
    },
    RecipeVariant {
        inputs: &TEXTILE_WOOL_INPUTS,
        outputs: &TEXTILE_OUTPUTS,
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
const LUMBER_VARIANTS: [RecipeVariant; 2] = [
    RecipeVariant {
        inputs: &LUMBER_INPUTS,
        outputs: &LUMBER_OUTPUTS,
    },
    RecipeVariant {
        inputs: &LUMBER_INPUTS,
        outputs: &PAPER_OUTPUTS,
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
const STEEL_VARIANTS: [RecipeVariant; 1] = [RecipeVariant {
    inputs: &STEEL_INPUTS,
    outputs: &STEEL_OUTPUTS,
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
const FOOD_VARIANTS: [RecipeVariant; 2] = [
    RecipeVariant {
        inputs: &FOOD_LIVESTOCK_INPUTS,
        outputs: &FOOD_OUTPUTS,
    },
    RecipeVariant {
        inputs: &FOOD_FISH_INPUTS,
        outputs: &FOOD_OUTPUTS,
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
const CLOTHING_VARIANTS: [RecipeVariant; 1] = [RecipeVariant {
    inputs: &CLOTHING_INPUTS,
    outputs: &CLOTHING_OUTPUTS,
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
const FURNITURE_VARIANTS: [RecipeVariant; 1] = [RecipeVariant {
    inputs: &FURNITURE_INPUTS,
    outputs: &FURNITURE_OUTPUTS,
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
    good: Good::Arms,
    amount: 1,
}];
const METAL_VARIANTS: [RecipeVariant; 2] = [
    RecipeVariant {
        inputs: &METAL_INPUTS,
        outputs: &HARDWARE_OUTPUTS,
    },
    RecipeVariant {
        inputs: &METAL_INPUTS,
        outputs: &ARMAMENT_OUTPUTS,
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
const REFINERY_VARIANTS: [RecipeVariant; 1] = [RecipeVariant {
    inputs: &REFINERY_INPUTS,
    outputs: &REFINERY_OUTPUTS,
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
const RAILYARD_VARIANTS: [RecipeVariant; 1] = [RecipeVariant {
    inputs: &RAILYARD_INPUTS,
    outputs: &RAILYARD_OUTPUTS,
}];
const RAILYARD_RECIPE: ProductionRecipe = ProductionRecipe {
    variants: &RAILYARD_VARIANTS,
};

// Note: Shipyard no longer has a production recipe as ships are constructed
// directly as entities, not as goods. See ships::construction module.

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
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
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
        buildings.insert(BuildingKind::Shipyard, Building::shipyard());
        Self { buildings }
    }

    pub fn get(&self, kind: BuildingKind) -> Option<Building> {
        self.buildings.get(&kind).copied()
    }

    pub fn insert(&mut self, building: Building) {
        self.buildings.insert(building.kind, building);
    }
}
