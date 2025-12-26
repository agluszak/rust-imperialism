use bevy::prelude::*;
use std::collections::HashMap;

use crate::{economy::goods::Good, resources::ResourceType};

/// Aggregated commodity buckets shown on the transport screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum TransportCommodity {
    Grain,
    Fruit,
    Fiber, // Cotton & Wool
    Meat,  // Livestock & Fish
    Timber,
    Coal,
    Iron,
    Precious, // Gold & Gems
    Oil,
    Fabric,
    Lumber,
    Paper,
    Steel,
    Fuel,
    Clothing,
    Furniture,
    Hardware,
    Armaments,
    CannedFood,
    Horses,
}

impl TransportCommodity {
    /// Goods that fall under this commodity bucket.
    pub fn goods(self) -> &'static [Good] {
        use TransportCommodity::*;
        match self {
            Grain => &[Good::Grain],
            Fruit => &[Good::Fruit],
            Fiber => &[Good::Cotton, Good::Wool],
            Meat => &[Good::Livestock, Good::Fish],
            Timber => &[Good::Timber],
            Coal => &[Good::Coal],
            Iron => &[Good::Iron],
            Precious => &[Good::Gold, Good::Gems],
            Oil => &[Good::Oil],
            Fabric => &[Good::Fabric],
            Lumber => &[Good::Lumber],
            Paper => &[Good::Paper],
            Steel => &[Good::Steel],
            Fuel => &[Good::Fuel],
            Clothing => &[Good::Clothing],
            Furniture => &[Good::Furniture],
            Hardware => &[Good::Hardware],
            Armaments => &[Good::Arms],
            CannedFood => &[Good::CannedFood],
            Horses => &[Good::Horses],
        }
    }

    /// Lookup the commodity bucket for a specific good.
    pub fn from_good(good: Good) -> Option<Self> {
        use Good::*;
        match good {
            Grain => Some(TransportCommodity::Grain),
            Fruit => Some(TransportCommodity::Fruit),
            Cotton | Wool => Some(TransportCommodity::Fiber),
            Livestock | Fish => Some(TransportCommodity::Meat),
            Timber => Some(TransportCommodity::Timber),
            Coal => Some(TransportCommodity::Coal),
            Iron => Some(TransportCommodity::Iron),
            Gold | Gems => Some(TransportCommodity::Precious),
            Oil => Some(TransportCommodity::Oil),
            Fabric => Some(TransportCommodity::Fabric),
            Lumber => Some(TransportCommodity::Lumber),
            Paper => Some(TransportCommodity::Paper),
            Steel => Some(TransportCommodity::Steel),
            Fuel => Some(TransportCommodity::Fuel),
            Clothing => Some(TransportCommodity::Clothing),
            Furniture => Some(TransportCommodity::Furniture),
            Hardware => Some(TransportCommodity::Hardware),
            Arms => Some(TransportCommodity::Armaments),
            CannedFood => Some(TransportCommodity::CannedFood),
            Horses => Some(TransportCommodity::Horses),
            Transport => None, // Transport converts to capacity, ships are entities
        }
    }

    /// Raw resource tiles that map to this commodity bucket.
    pub fn resource_types(self) -> &'static [ResourceType] {
        use TransportCommodity::*;
        match self {
            Grain => &[ResourceType::Grain],
            Fruit => &[ResourceType::Fruit],
            Fiber => &[ResourceType::Cotton, ResourceType::Wool],
            Meat => &[ResourceType::Livestock, ResourceType::Fish],
            Timber => &[ResourceType::Timber],
            Coal => &[ResourceType::Coal],
            Iron => &[ResourceType::Iron],
            Precious => &[ResourceType::Gold, ResourceType::Gems],
            Oil => &[ResourceType::Oil],
            // Manufactured goods do not map to tile resources directly.
            Fabric | Lumber | Paper | Steel | Fuel | Clothing | Furniture | Hardware
            | Armaments | CannedFood | Horses => &[],
        }
    }

    /// Icon bitmap path used for compact UI displays.
    pub fn icon(self) -> &'static str {
        use TransportCommodity::*;
        match self {
            Grain => "bitmaps/717.BMP",
            Fruit => "bitmaps/718.BMP",
            Fiber => "bitmaps/700.BMP", // Cotton
            Meat => "bitmaps/720.BMP",  // Cattle
            Timber => "bitmaps/702.BMP",
            Coal => "bitmaps/703.BMP",
            Iron => "bitmaps/704.BMP",
            Precious => "bitmaps/722.BMP", // Gold
            Oil => "bitmaps/706.BMP",
            Fabric => "bitmaps/708.BMP", // Wool (processed fiber)
            Lumber => "bitmaps/709.BMP",
            Paper => "bitmaps/710.BMP",
            Steel => "bitmaps/711.BMP",
            Fuel => "bitmaps/712.BMP",
            Clothing => "bitmaps/713.BMP",
            Furniture => "bitmaps/714.BMP",
            Hardware => "bitmaps/715.BMP",  // Tools
            Armaments => "bitmaps/716.BMP", // Arms
            CannedFood => "bitmaps/707.BMP",
            Horses => "bitmaps/705.BMP",
        }
    }

    /// Ordering used for UI layout: resources → materials → goods.
    pub const ORDERED: [TransportCommodity; 20] = [
        TransportCommodity::Grain,
        TransportCommodity::Fruit,
        TransportCommodity::Fiber,
        TransportCommodity::Meat,
        TransportCommodity::Timber,
        TransportCommodity::Coal,
        TransportCommodity::Iron,
        TransportCommodity::Precious,
        TransportCommodity::Oil,
        TransportCommodity::Fabric,
        TransportCommodity::Lumber,
        TransportCommodity::Paper,
        TransportCommodity::Steel,
        TransportCommodity::Fuel,
        TransportCommodity::Clothing,
        TransportCommodity::Furniture,
        TransportCommodity::Hardware,
        TransportCommodity::Armaments,
        TransportCommodity::CannedFood,
        TransportCommodity::Horses,
    ];
}

/// Total capacity available to each nation.
#[derive(Default, Resource, Debug, Clone)]
pub struct TransportCapacity {
    pub nations: HashMap<Entity, CapacitySnapshot>,
}

pub const BASE_TRANSPORT_CAPACITY: u32 = 5;

#[derive(Debug, Clone, Copy)]
pub struct CapacitySnapshot {
    pub total: u32,
    pub used: u32,
}

impl Default for CapacitySnapshot {
    fn default() -> Self {
        Self {
            total: BASE_TRANSPORT_CAPACITY,
            used: 0,
        }
    }
}

/// Desired allocations per nation and commodity.
#[derive(Default, Resource, Debug, Clone)]
pub struct TransportAllocations {
    pub nations: HashMap<Entity, NationAllocations>,
}

#[derive(Default, Debug, Clone)]
pub struct NationAllocations {
    pub commodities: HashMap<TransportCommodity, AllocationSlot>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct AllocationSlot {
    pub requested: u32,
    pub granted: u32,
}

/// Snapshot of supply/demand used for UI hints.
#[derive(Default, Resource, Debug, Clone)]
pub struct TransportDemandSnapshot {
    pub nations: HashMap<Entity, HashMap<TransportCommodity, DemandEntry>>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct DemandEntry {
    pub supply: u32,
    pub demand: u32,
}

impl NationAllocations {
    pub fn slot_mut(&mut self, commodity: TransportCommodity) -> &mut AllocationSlot {
        self.commodities.entry(commodity).or_default()
    }

    pub fn slot(&self, commodity: TransportCommodity) -> AllocationSlot {
        self.commodities
            .get(&commodity)
            .copied()
            .unwrap_or_default()
    }
}

impl TransportAllocations {
    pub fn ensure_nation(&mut self, nation: Entity) -> &mut NationAllocations {
        self.nations.entry(nation).or_default()
    }

    pub fn slot(&self, nation: Entity, commodity: TransportCommodity) -> AllocationSlot {
        self.nations
            .get(&nation)
            .map(|alloc| alloc.slot(commodity))
            .unwrap_or_default()
    }
}

impl TransportCapacity {
    pub fn snapshot_mut(&mut self, nation: Entity) -> &mut CapacitySnapshot {
        self.nations.entry(nation).or_default()
    }

    pub fn snapshot(&self, nation: Entity) -> CapacitySnapshot {
        self.nations.get(&nation).copied().unwrap_or_default()
    }
}
