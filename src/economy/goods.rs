use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Good {
    // Raw food resources
    Grain,
    Fruit,
    Livestock,
    Fish,

    // Fiber resources
    Cotton,
    Wool,

    // Other resources
    Timber,
    Coal,
    Iron,
    Gold,
    Gems,
    Oil,

    // Materials (2:1 from resources)
    Fabric, // from 2×(Cotton|Wool)
    Paper,  // from 2×Timber
    Lumber, // from 2×Timber
    Steel,  // from 1×Iron + 1×Coal
    Fuel,   // from 2×Oil

    // Goods (2:1 from materials)
    Clothing,   // from 2×Fabric
    Furniture,  // from 2×Lumber
    Hardware,   // from 2×Steel
    Armaments,  // from 2×Steel
    CannedFood, // from 2×Grain + 1×Fruit + 1×(Livestock|Fish)

    // Special
    Horses,
    Transport, // Freight cars for moving goods

    // Legacy (keeping for compatibility)
    Cloth, // Same as Fabric
}

impl Good {
    /// Returns true if this is a raw food resource (Grain, Fruit, Livestock, Fish)
    pub fn is_raw_food(self) -> bool {
        matches!(
            self,
            Good::Grain | Good::Fruit | Good::Livestock | Good::Fish
        )
    }

    /// Returns true if this is a resource (not processed)
    pub fn is_resource(self) -> bool {
        matches!(
            self,
            Good::Grain
                | Good::Fruit
                | Good::Livestock
                | Good::Fish
                | Good::Cotton
                | Good::Wool
                | Good::Timber
                | Good::Coal
                | Good::Iron
                | Good::Gold
                | Good::Gems
                | Good::Oil
        )
    }

    /// Returns true if this is a material (first-stage processing)
    pub fn is_material(self) -> bool {
        matches!(
            self,
            Good::Fabric | Good::Paper | Good::Lumber | Good::Steel | Good::Fuel
        )
    }

    /// Returns true if this is a finished good (second-stage processing)
    pub fn is_finished_good(self) -> bool {
        matches!(
            self,
            Good::Clothing | Good::Furniture | Good::Hardware | Good::Armaments | Good::CannedFood
        )
    }
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Good::Grain => write!(f, "Grain"),
            Good::Fruit => write!(f, "Fruit"),
            Good::Livestock => write!(f, "Livestock"),
            Good::Fish => write!(f, "Fish"),
            Good::Cotton => write!(f, "Cotton"),
            Good::Wool => write!(f, "Wool"),
            Good::Timber => write!(f, "Timber"),
            Good::Coal => write!(f, "Coal"),
            Good::Iron => write!(f, "Iron"),
            Good::Gold => write!(f, "Gold"),
            Good::Gems => write!(f, "Gems"),
            Good::Oil => write!(f, "Oil"),
            Good::Fabric => write!(f, "Fabric"),
            Good::Paper => write!(f, "Paper"),
            Good::Lumber => write!(f, "Lumber"),
            Good::Steel => write!(f, "Steel"),
            Good::Fuel => write!(f, "Fuel"),
            Good::Clothing => write!(f, "Clothing"),
            Good::Furniture => write!(f, "Furniture"),
            Good::Hardware => write!(f, "Hardware"),
            Good::Armaments => write!(f, "Armaments"),
            Good::CannedFood => write!(f, "Canned Food"),
            Good::Horses => write!(f, "Horses"),
            Good::Transport => write!(f, "Transport"),
            Good::Cloth => write!(f, "Cloth"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::economy::*;

    #[test]
    fn display_formats() {
        assert_eq!(Good::Wool.to_string(), "Wool");
        assert_eq!(Good::Cotton.to_string(), "Cotton");
        assert_eq!(Good::Cloth.to_string(), "Cloth");
        assert_eq!(Good::Grain.to_string(), "Grain");
        assert_eq!(Good::CannedFood.to_string(), "Canned Food");
        assert_eq!(Good::Furniture.to_string(), "Furniture");
    }

    #[test]
    fn raw_food_classification() {
        assert!(Good::Grain.is_raw_food());
        assert!(Good::Fruit.is_raw_food());
        assert!(Good::Livestock.is_raw_food());
        assert!(Good::Fish.is_raw_food());
        assert!(!Good::CannedFood.is_raw_food());
        assert!(!Good::Cotton.is_raw_food());
    }

    #[test]
    fn resource_classification() {
        assert!(Good::Grain.is_resource());
        assert!(Good::Timber.is_resource());
        assert!(Good::Iron.is_resource());
        assert!(!Good::Fabric.is_resource());
        assert!(!Good::Clothing.is_resource());
    }

    #[test]
    fn material_classification() {
        assert!(Good::Fabric.is_material());
        assert!(Good::Lumber.is_material());
        assert!(Good::Steel.is_material());
        assert!(!Good::Cotton.is_material());
        assert!(!Good::Clothing.is_material());
    }

    #[test]
    fn finished_good_classification() {
        assert!(Good::Clothing.is_finished_good());
        assert!(Good::Furniture.is_finished_good());
        assert!(Good::CannedFood.is_finished_good());
        assert!(!Good::Fabric.is_finished_good());
        assert!(!Good::Grain.is_finished_good());
    }
}
