# Allocation UI Widgets Design

## Problem

Currently, the Capitol dialog has inline code for allocation controls. We need reusable widgets for:
- Trade School (training allocation)
- All production buildings (production allocation)
- Future features (construction, trade orders, etc.)

## Design Goals

1. **Minimal code duplication** - Common patterns extracted into widgets
2. **Type-safe** - Each widget knows what it controls
3. **Composable** - Mix and match widgets as needed
4. **Consistent UX** - All allocation UIs look and behave the same

## Widget Catalog

### 1. Allocation Stepper
**Purpose**: Adjust a numeric value with +/- buttons

**Visual**:
```
[-5] [-1] [current value] [+1] [+5]
```

**Usage**:
```rust
spawn_allocation_stepper(
    parent,
    "Recruit Workers",
    StepperConfig {
        small_step: 1,
        large_step: 5,
        message_type: StepperMessage::Recruitment,
    }
);
```

**Components**:
- `AllocationStepperDisplay { id }` - Marks the value display text
- `AllocationStepperButton { id, delta }` - Marks +/- buttons

### 2. Allocation Bar
**Purpose**: Show allocated vs available for a resource

**Visual**:
```
Canned Food: 5 / 10 [████░░░░░░]
```

**Usage**:
```rust
spawn_allocation_bar(
    parent,
    "Canned Food",
    AllocationBarConfig {
        good: Good::CannedFood,
        bar_type: BarType::Recruitment,
    }
);
```

**Components**:
- `AllocationBar { good, bar_type }` - Marks the bar container
- Bar updates via rendering system based on ResourceAllocations

### 3. Allocation Summary
**Purpose**: Show "Will do X next turn" summary

**Visual**:
```
→ Will recruit 5 workers next turn
```

**Usage**:
```rust
spawn_allocation_summary(
    parent,
    AllocationSummaryConfig {
        summary_type: SummaryType::Recruitment,
    }
);
```

**Components**:
- `AllocationSummary { summary_type }` - Marks the summary text

### 4. Resource Requirements List
**Purpose**: Show per-unit requirements (non-interactive)

**Visual**:
```
Requirements per worker:
  • 1× Canned Food ✓
  • 1× Clothing ✓
  • 1× Furniture ✗
```

**Usage**:
```rust
spawn_requirements_list(
    parent,
    vec![
        (Good::CannedFood, 1),
        (Good::Clothing, 1),
        (Good::Furniture, 1),
    ]
);
```

## Unified Component System

### Core Components

```rust
/// Identifies what this allocation UI controls
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationType {
    Recruitment,
    Training(WorkerSkill),
    Production(Entity), // building entity
}

/// Generic stepper display (shows current allocated value)
#[derive(Component)]
pub struct AllocationStepperDisplay {
    pub allocation_type: AllocationType,
}

/// Generic stepper button
#[derive(Component)]
pub struct AllocationStepperButton {
    pub allocation_type: AllocationType,
    pub delta: i32,
}

/// Generic allocation bar
#[derive(Component)]
pub struct AllocationBar {
    pub allocation_type: AllocationType,
    pub good: Good,
}

/// Generic summary text
#[derive(Component)]
pub struct AllocationSummary {
    pub allocation_type: AllocationType,
}
```

### Unified Update Systems

```rust
/// Single system updates ALL stepper displays
pub fn update_all_stepper_displays(
    allocations: Query<&ResourceAllocations>,
    mut displays: Query<(&mut Text, &AllocationStepperDisplay)>,
) {
    for (mut text, display) in displays.iter_mut() {
        let value = match display.allocation_type {
            AllocationType::Recruitment => alloc.recruitment.allocated,
            AllocationType::Training(skill) => {
                alloc.training.iter()
                    .find(|t| t.from_skill == skill)
                    .map(|t| t.allocated)
                    .unwrap_or(0)
            }
            AllocationType::Production(entity) => {
                alloc.production.get(&entity)
                    .map(|p| p.allocated_output)
                    .unwrap_or(0)
            }
        };
        text.0 = format!("{}", value);
    }
}

/// Single system handles ALL stepper button clicks
pub fn handle_all_stepper_buttons(
    interactions: Query<(&Interaction, &AllocationStepperButton), Changed<Interaction>>,
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&ResourceAllocations>,
    mut recruit_writer: MessageWriter<AdjustRecruitment>,
    mut train_writer: MessageWriter<AdjustTraining>,
    mut prod_writer: MessageWriter<AdjustProduction>,
) {
    // Single handler dispatches to appropriate message based on allocation_type
}

/// Single system updates ALL allocation bars
pub fn update_all_allocation_bars(
    allocations: Query<&ResourceAllocations>,
    stockpiles: Query<&Stockpile>,
    mut bars: Query<(&mut BackgroundColor, &mut BorderColor, &AllocationBar)>,
) {
    // Single handler computes allocated/available for any type
}
```

## Widget Spawn Functions

### Pattern: Builder-Style Config

```rust
pub struct StepperConfig {
    pub label: &'static str,
    pub allocation_type: AllocationType,
    pub small_step: i32,
    pub large_step: i32,
}

pub fn spawn_allocation_stepper(
    parent: &mut ChildBuilder,
    config: StepperConfig,
) {
    parent.spawn((
        Text::new(config.label),
        // ... styling ...
    ));

    parent.spawn(Node { /* stepper row */ })
        .with_children(|row| {
            // -large button
            spawn_stepper_button(row, config.allocation_type, -config.large_step);
            // -small button
            spawn_stepper_button(row, config.allocation_type, -config.small_step);

            // Display
            row.spawn((
                Text::new("0"),
                AllocationStepperDisplay { allocation_type: config.allocation_type },
                // ... styling ...
            ));

            // +small button
            spawn_stepper_button(row, config.allocation_type, config.small_step);
            // +large button
            spawn_stepper_button(row, config.allocation_type, config.large_step);
        });
}

fn spawn_stepper_button(
    parent: &mut ChildBuilder,
    allocation_type: AllocationType,
    delta: i32,
) {
    let label = if delta > 0 {
        format!("+{}", delta)
    } else {
        format!("{}", delta)
    };

    parent.spawn((
        Button,
        AllocationStepperButton { allocation_type, delta },
        // ... styling ...
    ))
    .with_children(|btn| {
        btn.spawn((Text::new(label), /* ... */));
    });
}
```

### Pattern: Chained Spawners

```rust
pub struct AllocationBarConfig {
    pub good: Good,
    pub good_name: &'static str,
    pub allocation_type: AllocationType,
}

pub fn spawn_allocation_bar(
    parent: &mut ChildBuilder,
    config: AllocationBarConfig,
) {
    parent.spawn(Node { /* container */ })
        .with_children(|container| {
            // Label
            container.spawn((
                Text::new(format!("{}: 0 / 0", config.good_name)),
                AllocationBar {
                    allocation_type: config.allocation_type,
                    good: config.good,
                },
                // ... styling ...
            ));

            // Bar visual
            container.spawn((
                Node { /* bar node */ },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8)),
                // ... styling ...
            ));
        });
}
```

## Example Usage: Capitol

**Before** (inline code):
```rust
// 60+ lines of inline button and bar spawning
```

**After** (using widgets):
```rust
fn spawn_capitol_content(commands: &mut Commands, content_entity: Entity, /* ... */) {
    commands.entity(content_entity).with_children(|content| {
        content.spawn(Text::new("Worker Recruitment"));

        // Requirements section (non-interactive, keep as-is)
        spawn_requirements_section(content, /* ... */);

        // Stepper
        spawn_allocation_stepper(content, StepperConfig {
            label: "Allocate Workers",
            allocation_type: AllocationType::Recruitment,
            small_step: 1,
            large_step: 5,
        });

        // Resource bars
        for (good, name) in [
            (Good::CannedFood, "Canned Food"),
            (Good::Clothing, "Clothing"),
            (Good::Furniture, "Furniture"),
        ] {
            spawn_allocation_bar(content, AllocationBarConfig {
                good,
                good_name: name,
                allocation_type: AllocationType::Recruitment,
            });
        }

        // Summary
        spawn_allocation_summary(content, AllocationSummaryConfig {
            allocation_type: AllocationType::Recruitment,
        });
    });
}
```

## Example Usage: Trade School

```rust
fn spawn_trade_school_content(/* ... */) {
    commands.entity(content_entity).with_children(|content| {
        content.spawn(Text::new("Worker Training"));

        // Current workforce display (non-interactive, keep as-is)
        spawn_workforce_display(content, workforce);

        // Untrained → Trained section
        content.spawn(Text::new("Train Untrained → Trained"));
        spawn_allocation_stepper(content, StepperConfig {
            label: "Allocate",
            allocation_type: AllocationType::Training(WorkerSkill::Untrained),
            small_step: 1,
            large_step: 5,
        });
        spawn_allocation_bar(content, AllocationBarConfig {
            good: Good::Paper,
            good_name: "Paper",
            allocation_type: AllocationType::Training(WorkerSkill::Untrained),
        });

        // Trained → Expert section (same pattern)
        // ...
    });
}
```

## Example Usage: Production (Textile Mill)

```rust
fn spawn_production_dialog_content(/* ... */) {
    commands.entity(content_entity).with_children(|content| {
        content.spawn(Text::new("Textile Mill"));

        // Choice buttons (Use Cotton / Use Wool)
        spawn_production_choice_buttons(content, building_entity);

        // Output stepper
        spawn_allocation_stepper(content, StepperConfig {
            label: "Target Output",
            allocation_type: AllocationType::Production(building_entity),
            small_step: 1,
            large_step: capacity / 4,
        });

        // Input bars (dynamic based on choice)
        spawn_production_input_bars(content, building_entity);

        // Summary
        spawn_allocation_summary(content, AllocationSummaryConfig {
            allocation_type: AllocationType::Production(building_entity),
        });
    });
}
```

## Benefits

1. **~80% less code** in dialog spawners
2. **Single source of truth** for stepper/bar styling
3. **Unified update systems** - one system handles all steppers
4. **Easy to extend** - add new AllocationType variants
5. **Consistent UX** - all allocation UIs look identical

## Implementation Plan

1. Create `src/ui/city/allocation_widgets.rs`
2. Define `AllocationType` enum and unified components
3. Implement spawn functions (stepper, bar, summary)
4. Implement unified update/input systems
5. Refactor Capitol to use widgets
6. Refactor Trade School to use widgets
7. Update production dialogs to use widgets
8. Test all UIs end-to-end

## File Structure

```
src/ui/city/
├── allocation_widgets.rs     # NEW: Widget spawn functions + unified components
├── allocation_ui.rs           # KEEP: Unified update/input systems (refactored)
├── components.rs              # UPDATE: Remove per-dialog components, add AllocationType
├── dialogs/
│   └── special.rs             # UPDATE: Use widgets instead of inline code
```
