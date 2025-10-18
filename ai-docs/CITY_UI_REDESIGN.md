# City UI Redesign Plan

## Current vs. New Architecture

### Current (layout.rs: 816 lines, all inline)
```
┌─────────────────────────────────────────────┐
│ [Back to Map]                               │
│                                             │
│ ┌─────────────┐  ┌──────────────────────┐ │
│ │ Workforce   │  │ Warehouse            │ │
│ │ Panel       │  │ Panel                │ │
│ └─────────────┘  └──────────────────────┘ │
│                                             │
│ ┌───────────────────────────────────────┐  │
│ │ Textile Mill Panel                    │  │
│ │  [Use Cotton] [Use Wool]              │  │
│ │  [-] 4 [+]                            │  │
│ └───────────────────────────────────────┘  │
│                                             │
│ ┌───────────────────────────────────────┐  │
│ │ Lumber Mill Panel                     │  │
│ └───────────────────────────────────────┘  │
│                                             │
│ ┌───────────────────────────────────────┐  │
│ │ Civilian Hiring Grid (3×3)            │  │
│ └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### New (Imperialism 1997 style)
```
┌─────────────────────────────────────────────┐
│ ┌─────────────┐ [Warehouse] ┌────────────┐ │
│ │ LABOR POOL  │  (top HUD)  │ FOOD       │ │
│ │ Available:  │             │ DEMAND     │ │
│ │ 12 labor    │             │ Grain: 5   │ │
│ │             │             │ Fruit: 3   │ │
│ │ Untrained:5 │             └────────────┘ │
│ │ Trained: 3  │                            │
│ │ Expert: 1   │                            │
│ │ (left HUD)  │  ┌──────┐ ┌──────┐       │ │
│ │             │  │[TEX] │ │[LUM] │       │ │
│ │             │  │TILE  │ │BER   │       │ │
│ │             │  │MILL  │ │MILL  │       │ │
│ │             │  └──────┘ └──────┘       │ │
│ │             │  ┌──────┐ ┌──────┐       │ │
│ │             │  │[CAP] │ │[TRD] │       │ │
│ │             │  │ITOL  │ │SCH   │       │ │
│ │             │  └──────┘ └──────┘       │ │
│ └─────────────┘                            │
│                                             │
│ ┌────────────────────────────────────────┐ │
│ │ TEXTILE MILL                    [Close]│ │
│ │ ┌────────────────────────────────────┐ │ │
│ │ │ [Cotton][Cotton] → [Fabric]      X │ │ │
│ │ └────────────────────────────────────┘ │ │
│ │ Output: [||||----] 4 / 8 (Capacity)   │ │
│ │         [-] 4 [+]                      │ │
│ │ Labor: 4 units                         │ │
│ │ [Expand Industry] (costs 1L + 1S)     │ │
│ └────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

## New Component Structure

### 1. Persistent HUD Components
```rust
// Left border
#[derive(Component)]
pub struct LaborPoolPanel;

#[derive(Component)]
pub struct AvailableLaborDisplay; // Updates live

#[derive(Component)]
pub struct WorkforceCountDisplay; // Shows untrained/trained/expert

// Right border
#[derive(Component)]
pub struct FoodDemandPanel;

#[derive(Component)]
pub struct FoodDemandDisplay; // Shows required food by type

// Top center
#[derive(Component)]
pub struct WarehouseHUD; // Compact read-only display

#[derive(Component)]
pub struct WarehouseStockText; // Updates live
```

### 2. Building Grid
```rust
#[derive(Component)]
pub struct BuildingGrid; // Container for all building buttons

#[derive(Component)]
pub struct BuildingButton {
    pub building_entity: Option<Entity>, // None if not built yet
    pub building_kind: BuildingKind,
}
```

### 3. Building Dialog System
```rust
// Dialog overlay (modal window)
#[derive(Component)]
pub struct BuildingDialog {
    pub building_entity: Entity,
    pub building_kind: BuildingKind,
}

// Dialog components
#[derive(Component)]
pub struct ProductionEquation; // Visual: [Input][Input] → [Output]

#[derive(Component)]
pub struct MissingInputIndicator; // Red X overlay

#[derive(Component)]
pub struct CapacitySlider {
    pub building_entity: Entity,
}

#[derive(Component)]
pub struct ExpandIndustryButton {
    pub building_entity: Entity,
}

// Multiple dialogs can be open at once (windowed system)
```

## New File Structure

```
src/ui/city/
├── mod.rs              # Plugin registration
├── components.rs       # All component types & messages
├── hud/                # NEW: Persistent HUD borders
│   ├── mod.rs
│   ├── labor.rs        # Labor pool panel + updates
│   ├── food.rs         # Food demand panel + updates
│   └── warehouse.rs    # Compact warehouse HUD + updates
├── buildings/          # NEW: Building grid & buttons
│   ├── mod.rs
│   ├── grid.rs         # Building button grid layout
│   └── buttons.rs      # Building button input handlers
├── dialogs/            # NEW: Building dialog system
│   ├── mod.rs
│   ├── types.rs        # Dialog component types
│   ├── production.rs   # Production building dialogs (mills/factories)
│   ├── workforce.rs    # Capitol, Trade School, University
│   ├── infrastructure.rs # Railyard, Shipyard, Armory
│   └── shared.rs       # Shared dialog widgets (equation, slider, expand)
├── production.rs       # Keep: production logic handlers
└── warehouse.rs        # Remove: merged into hud/warehouse.rs
```

## Implementation Phases

### Phase 1: HUD Borders (Labor, Food, Warehouse)
- Create persistent left/right/top panels
- Move labor display from inline to left border
- Add food demand display to right border
- Convert warehouse to compact top HUD
- All update systems stay similar but target new components

### Phase 2: Building Grid
- Replace inline building panels with button grid
- Each button shows building icon + name
- Click handler opens dialog
- Grid layout: 3-4 columns, rows as needed

### Phase 3: Dialog System Foundation
- Create modal dialog overlay system
- Add close button
- Support multiple open dialogs (windowing)
- Z-index management for overlapping dialogs

### Phase 4: Production Dialogs
- TextileMill, LumberMill, SteelMill, FoodProcessing dialogs
- Production equation display with icons
- Missing input indicator (red X)
- Capacity slider
- Labor cost display
- Expand Industry button

### Phase 5: Special Building Dialogs
- Capitol: recruitment UI
- Trade School: training UI
- Railyard/Shipyard/Armory: unit building UIs

## Key Behavior Changes

### Live Feedback
- Adjusting production slider immediately:
  - Decrements available labor display
  - Shows required vs. available inputs
  - Updates warehouse display
- Multiple dialogs open = you can balance across buildings in one pass

### Persistent Orders
- Dialog settings persist when closed
- Reopen shows current settings
- Execute on End Turn

### Missing Input UX
- Red X appears over input icons when stock < required
- Trade screen integration (future): shortage icon for bidding hints

## Migration Strategy

1. Create new file structure alongside old
2. Implement Phase 1 (HUD) with new components
3. Keep old layout.rs working during development
4. Replace old system incrementally
5. Remove old layout.rs when complete

## Compatibility Notes

- Keep all existing messages (HireCivilian, ChangeProductionSettings, etc.)
- Logic handlers (production.rs, workforce.rs logic) stay unchanged
- Only UI presentation layer changes
- All tests should continue passing
