use bevy::prelude::*;

use crate::economy::goods::Good;

/// Types of resources that can be found/developed on tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    // Agriculture
    Grain,
    Fruit,
    Cotton,
    // Livestock
    Wool,
    Livestock,
    Fish, // Coastal fisheries gathered by ports
    // Natural
    Timber,
    // Minerals (must be discovered by Prospector)
    Coal,
    Iron,
    Gold,
    Gems,
    // Oil (requires Oil Drilling tech to prospect)
    Oil,
}

impl ResourceType {
    /// Convert a ResourceType to its corresponding Good for stockpile storage
    pub fn to_good(self) -> Good {
        match self {
            ResourceType::Grain => Good::Grain,
            ResourceType::Fruit => Good::Fruit,
            ResourceType::Cotton => Good::Cotton,
            ResourceType::Wool => Good::Wool,
            ResourceType::Livestock => Good::Livestock,
            ResourceType::Fish => Good::Fish,
            ResourceType::Timber => Good::Timber,
            ResourceType::Coal => Good::Coal,
            ResourceType::Iron => Good::Iron,
            ResourceType::Gold => Good::Gold,
            ResourceType::Gems => Good::Gems,
            ResourceType::Oil => Good::Oil,
        }
    }
}

/// Static list of all resource types for easy iteration.
pub const ALL_RESOURCES: &[ResourceType] = &[
    ResourceType::Grain,
    ResourceType::Fruit,
    ResourceType::Cotton,
    ResourceType::Wool,
    ResourceType::Livestock,
    ResourceType::Fish,
    ResourceType::Timber,
    ResourceType::Coal,
    ResourceType::Iron,
    ResourceType::Gold,
    ResourceType::Gems,
    ResourceType::Oil,
];

impl ResourceType {
    /// Returns true when the resource provides a baseline yield without improvements.
    pub fn is_baseline_yield_eligible(self) -> bool {
        matches!(
            self,
            ResourceType::Grain
                | ResourceType::Fruit
                | ResourceType::Cotton
                | ResourceType::Wool
                | ResourceType::Livestock
                | ResourceType::Fish
                | ResourceType::Timber
        )
    }
}

/// Development level of a resource (0-3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DevelopmentLevel {
    Lv0 = 0, // Undeveloped
    Lv1 = 1, // Basic development
    Lv2 = 2, // Improved
    Lv3 = 3, // Fully developed
}

/// Component marking a tile as having a resource
#[derive(Component, Debug, Clone, Copy)]
pub struct TileResource {
    pub resource_type: ResourceType,
    pub development: DevelopmentLevel,
    pub discovered: bool, // Minerals start false, must be discovered by Prospector
}

impl TileResource {
    /// Create a new agricultural/natural resource (visible by default)
    pub fn visible(resource_type: ResourceType) -> Self {
        Self {
            resource_type,
            development: DevelopmentLevel::Lv0,
            discovered: true,
        }
    }

    /// Returns true if this resource type must be prospected by each nation
    pub fn requires_prospecting(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Coal
                | ResourceType::Iron
                | ResourceType::Gold
                | ResourceType::Gems
                | ResourceType::Oil
        )
    }

    /// Create a new mineral resource (hidden until discovered)
    pub fn hidden_mineral(resource_type: ResourceType) -> Self {
        Self {
            resource_type,
            development: DevelopmentLevel::Lv0,
            discovered: false,
        }
    }

    /// Get per-turn output based on resource type and development level
    pub fn get_output(&self) -> u32 {
        if !self.discovered {
            return 0;
        }

        if self.resource_type.is_baseline_yield_eligible() {
            // Food/fiber/timber: 1/2/3/4
            return match self.development {
                DevelopmentLevel::Lv0 => 1,
                DevelopmentLevel::Lv1 => 2,
                DevelopmentLevel::Lv2 => 3,
                DevelopmentLevel::Lv3 => 4,
            };
        }

        match self.resource_type {
            // Coal/iron/oil: 0/2/4/6
            ResourceType::Coal | ResourceType::Iron | ResourceType::Oil => match self.development {
                DevelopmentLevel::Lv0 => 0,
                DevelopmentLevel::Lv1 => 2,
                DevelopmentLevel::Lv2 => 4,
                DevelopmentLevel::Lv3 => 6,
            },
            // Gold/gems: 0/1/2/3
            ResourceType::Gold | ResourceType::Gems => match self.development {
                DevelopmentLevel::Lv0 => 0,
                DevelopmentLevel::Lv1 => 1,
                DevelopmentLevel::Lv2 => 2,
                DevelopmentLevel::Lv3 => 3,
            },
            _ => 0,
        }
    }

    /// Check if this resource can be improved by a Farmer
    pub fn improvable_by_farmer(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Grain | ResourceType::Fruit | ResourceType::Cotton
        )
    }

    /// Check if this resource can be improved by a Rancher
    pub fn improvable_by_rancher(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Wool | ResourceType::Livestock
        )
    }

    /// Check if this resource can be improved by a Forester
    pub fn improvable_by_forester(&self) -> bool {
        matches!(self.resource_type, ResourceType::Timber)
    }

    /// Check if this resource can be improved by a Miner
    pub fn improvable_by_miner(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Coal | ResourceType::Iron | ResourceType::Gold | ResourceType::Gems
        )
    }

    /// Check if this resource can be improved by a Driller
    pub fn improvable_by_driller(&self) -> bool {
        matches!(self.resource_type, ResourceType::Oil)
    }

    /// Improve development level (returns true if improved)
    pub fn improve(&mut self) -> bool {
        if !self.discovered {
            return false;
        }

        match self.development {
            DevelopmentLevel::Lv0 => {
                self.development = DevelopmentLevel::Lv1;
                true
            }
            DevelopmentLevel::Lv1 => {
                self.development = DevelopmentLevel::Lv2;
                true
            }
            DevelopmentLevel::Lv2 => {
                self.development = DevelopmentLevel::Lv3;
                true
            }
            DevelopmentLevel::Lv3 => false, // Already max level
        }
    }
}
