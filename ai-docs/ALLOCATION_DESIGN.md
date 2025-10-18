# Allocation System Design

## Problem Statement

Imperialism uses a **resource allocation model** where:
- Resources are **pre-allocated** for orders (not spent instantly)
- UI shows **allocation bars** (allocated/total available)
- Player can **adjust allocations** freely within the turn
- On **turn end**, allocated resources convert to outputs

Current codebase uses **immediate reservation on button click**, which doesn't match the source material.

## Core Concepts

### 1. Allocation vs Reservation

- **Allocation** (new): Player intent, visible in UI, adjustable during PlayerTurn
- **Reservation** (existing): Already in `Stockpile`, used for locking resources during Processing phase

**Flow**:
```
PlayerTurn: Player adjusts allocation → UI updates bars
          ↓
End Turn:   Allocations → Reservations (lock resources for processing)
          ↓
Processing: Consume reserved resources → produce outputs
          ↓
Next Turn:  Clear reservations, new allocations start fresh
```

### 2. Allocation Components

```rust
/// Per-nation component tracking all resource allocations for the current turn
#[derive(Component, Debug, Clone, Default)]
pub struct ResourceAllocations {
    /// Allocations for each type of order
    pub recruitment: RecruitmentAllocation,
    pub training: Vec<TrainingAllocation>,
    pub production: HashMap<Entity, ProductionAllocation>, // keyed by Building entity
}

#[derive(Debug, Clone, Default)]
pub struct RecruitmentAllocation {
    /// How many workers the player wants to recruit
    pub requested: u32,
    /// How many can actually be allocated given resources
    pub allocated: u32,
    /// Per-unit input requirements
    pub inputs_per_unit: Vec<(Good, u32)>, // [(CannedFood, 1), (Clothing, 1), (Furniture, 1)]
}

#[derive(Debug, Clone)]
pub struct TrainingAllocation {
    pub from_skill: WorkerSkill,
    /// How many workers to train
    pub requested: u32,
    pub allocated: u32,
    /// Per-unit input requirements
    pub inputs_per_unit: Vec<(Good, u32)>, // [(Paper, 1)]
    pub cash_per_unit: i64, // 100
}

#[derive(Debug, Clone)]
pub struct ProductionAllocation {
    pub building_kind: BuildingKind,
    pub choice: ProductionChoice,
    /// Target output (what player requested)
    pub target_output: u32,
    /// Actual allocated output (limited by inputs, capacity, labor)
    pub allocated_output: u32,
    /// Input requirements for the allocated amount
    pub inputs_needed: Vec<(Good, u32)>,
}
```

### 3. Messages (Input Layer)

```rust
/// Player adjusts recruitment slider
#[derive(Message)]
pub struct AdjustRecruitment {
    pub nation: Entity,
    pub requested: u32,
}

/// Player adjusts training slider for a skill level
#[derive(Message)]
pub struct AdjustTraining {
    pub nation: Entity,
    pub from_skill: WorkerSkill,
    pub requested: u32,
}

/// Player adjusts production settings (choice + target)
#[derive(Message)]
pub struct AdjustProduction {
    pub nation: Entity,
    pub building: Entity,
    pub choice: ProductionChoice,
    pub target_output: u32,
}
```

### 4. Systems (Logic Layer)

#### During PlayerTurn (Update schedule)

```rust
/// Reads AdjustRecruitment messages, updates ResourceAllocations.recruitment
/// Computes allocated = min(requested, capacity_cap, resources_available)
fn apply_recruitment_adjustments(
    mut messages: MessageReader<AdjustRecruitment>,
    mut nations: Query<(&mut ResourceAllocations, &Stockpile, &Workforce)>,
    provinces: Query<&Province>,
) {
    // For each message:
    // 1. Update requested
    // 2. Compute capacity cap (provinces/4 or /3)
    // 3. Compute resource cap (min of CannedFood, Clothing, Furniture available)
    // 4. Set allocated = min(requested, capacity_cap, resource_cap)
}

/// Similar for training and production
fn apply_training_adjustments(...) { }
fn apply_production_adjustments(...) { }
```

#### On Turn End (before phase transition to Processing)

```rust
/// Converts all allocations to stockpile reservations
/// This locks the resources for consumption during Processing
fn finalize_allocations(
    mut nations: Query<(&ResourceAllocations, &mut Stockpile, &mut RecruitmentQueue, &mut TrainingQueue)>,
    mut buildings: Query<(&mut ProductionSettings, &Building)>,
) {
    for (allocations, mut stockpile, mut recruit_queue, mut train_queue) in nations.iter_mut() {
        // 1. Recruitment: reserve resources, set queue.queued = allocated
        let r = &allocations.recruitment;
        for (good, qty_per) in &r.inputs_per_unit {
            stockpile.reserve(*good, qty_per * r.allocated);
        }
        recruit_queue.queued = r.allocated;

        // 2. Training: similar
        for t in &allocations.training {
            for (good, qty_per) in &t.inputs_per_unit {
                stockpile.reserve(*good, qty_per * t.allocated);
            }
            train_queue.add_order(t.from_skill, t.allocated);
        }

        // 3. Production: update ProductionSettings, reserve inputs
        for (building_entity, prod_alloc) in &allocations.production {
            if let Ok((mut settings, building)) = buildings.get_mut(*building_entity) {
                settings.choice = prod_alloc.choice;
                settings.target_output = prod_alloc.allocated_output;

                for (good, qty) in &prod_alloc.inputs_needed {
                    stockpile.reserve(*good, *qty);
                }
            }
        }
    }
}
```

#### Start of Next PlayerTurn

```rust
/// Clears all allocations to start fresh
fn reset_allocations(
    mut nations: Query<&mut ResourceAllocations>,
) {
    for mut alloc in nations.iter_mut() {
        *alloc = ResourceAllocations::default();
    }
}
```

### 5. UI (Rendering Layer)

Instead of buttons, we need:
- **Sliders** or **+/- steppers** to adjust `requested` values
- **Allocation bars** showing `allocated / total_available`
- **Color coding**: green (can allocate more), yellow (at limit), red (insufficient resources)

Example for recruitment:
```
┌─ Capitol Building ──────────────────────────────┐
│ Recruit Untrained Workers                       │
│                                                  │
│ Requested: [  -  ] 5 [  +  ]                   │
│                                                  │
│ Canned Food:  [████████░░] 5/10                │
│ Clothing:     [████████░░] 5/8                 │
│ Furniture:    [██████████] 5/5  ⚠️             │
│                                                  │
│ Will recruit: 5 workers next turn               │
│ (Limited by Furniture availability)             │
└──────────────────────────────────────────────────┘
```

## Migration Path

### Phase 1: Add Allocation Components (non-breaking)
- Add `ResourceAllocations` component to nations
- Add adjustment messages
- Keep existing queue/button systems working

### Phase 2: Implement Allocation Logic
- Add adjustment handler systems
- Add finalization system (runs on turn end)
- Add reset system (runs on turn start)

### Phase 3: Update UI
- Replace instant buttons with sliders/steppers
- Add allocation bars
- Wire up adjustment messages

### Phase 4: Remove Old System
- Remove direct queue manipulation from button handlers
- Remove immediate reservation calls
- Clean up old messages

## Testing Strategy

1. **Unit tests** for allocation calculation logic
   - Cap by resources
   - Cap by capacity (recruitment)
   - Cap by labor (production)

2. **Integration tests** for turn transitions
   - Allocations → Reservations → Consumption
   - Reset on new turn

3. **UI tests** (manual)
   - Slider adjustments update bars in real-time
   - Over-allocation shows red/warning
   - Turn end → resources consumed correctly

## Benefits

1. **Matches source material**: True allocation model like Imperialism
2. **Better UX**: Visual feedback on resource constraints
3. **Flexibility**: Change your mind freely during turn
4. **Clarity**: See exactly what will happen next turn
5. **Scalability**: Easy to add new allocation types (construction, trade orders, etc.)

## Open Questions

1. **Treasury allocations**: Do we need `reserved_cash` on Treasury component?
2. **Labor allocations**: Should labor be "allocated" to buildings, or computed on-demand?
3. **Multi-building production**: How to handle when player has multiple Textile Mills?
4. **Default allocations**: Should allocations persist from last turn as starting values?
