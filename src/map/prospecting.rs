use bevy::prelude::*;

use crate::resources::ResourceType;

/// Component marking a tile as having potential hidden minerals
/// The actual resource type is not known until prospecting completes
#[derive(Component, Debug, Clone, Copy)]
pub struct PotentialMineral {
    /// Hidden resource type (only used internally during prospecting)
    pub(crate) hidden_resource: Option<ResourceType>,
}

impl PotentialMineral {
    /// Create a potential mineral deposit that may or may not contain a resource
    pub fn new(resource: Option<ResourceType>) -> Self {
        Self {
            hidden_resource: resource,
        }
    }

    /// Check if this tile actually has a mineral (only called during prospecting)
    pub(crate) fn reveal(&self) -> Option<ResourceType> {
        self.hidden_resource
    }
}

/// Component marking a tile as prospected with no mineral found
#[derive(Component, Debug, Clone, Copy)]
pub struct ProspectedEmpty;

/// Component marking a tile as prospected and confirmed to have a mineral
#[derive(Component, Debug, Clone, Copy)]
pub struct ProspectedMineral {
    pub resource_type: ResourceType,
}
