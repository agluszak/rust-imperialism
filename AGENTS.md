# CLAUDE.md

Economy-first, turn-based strategy game inspired by Imperialism (1997). Built with Bevy 0.18 ECS, hex-based maps, multi-nation economies. Reference `manual.pdf` for game mechanics.

**Tech stack:** Bevy 0.18, `bevy_ecs_tilemap` 0.18, `hexx` 0.23, `moonshine-save` for serialization.

## Architecture

**Plugin-based:** Each subsystem has its own plugin in `mod.rs` (Economy, Map, Camera, Civilians, Diplomacy, AI, UI, Save). `lib.rs` only orchestrates plugins.

**Three-layer separation:**
```
Input Layer (observers, events) → Logic Layer (systems, state) → Rendering Layer (visuals)
```
- Input never mutates state directly
- Logic never queries UI interaction
- Messages (`MessageWriter`/`MessageReader`) decouple layers

**ECS patterns:**
- Per-nation data: Components on nation entities (`Stockpile`, `Treasury`, `Technologies`)
- Global state: Resources (`Calendar`, `TurnCounter`, `PlayerNation`)
- Turn phase: Bevy State (`TurnPhase`) with `OnEnter` schedules
- Map visuals: `MapTilemap` marker for automatic show/hide

**Turn loop:** PlayerTurn → Processing → EnemyTurn (auto-transitions). Use `OnEnter(TurnPhase::*)` for turn-based systems.

**Allocation system:** Pre-allocation model - reserve during PlayerTurn, commit at turn end, consume during Processing. See `ai-docs/ALLOCATION_DESIGN.md`.

## Code Conventions

**Imports:** Use explicit `crate::` paths everywhere (no `super::`). Group: std → external → crate.

**Modules:** Complex modules use subdirectories. Plugins always in `mod.rs`.

**Testing:** Small tests (<50 lines) inline, large tests in separate `tests.rs`.

**UI Buttons (Bevy 0.18):** Must use BOTH `Button` + `OldButton`. Import `Button` from `bevy::ui_widgets` and alias `bevy::ui::widget::Button as OldButton`. Use `.observe()` as builder method:
```rust
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button};

parent
    .spawn((
        Button,
        OldButton,
        Node { padding: UiRect::all(Val::Px(8.0)), ..default() },
        BackgroundColor(NORMAL_BUTTON),
    ))
    .observe(my_button_handler)
    .with_children(|p| { p.spawn((Text::new("Label"), ...)); });

fn my_button_handler(trigger: On<Activate>, /* params */) {
    let target = trigger.event().entity;
    // handler
}
```

## Development Guidelines

- **No backwards compatibility**: Don't make components optional for old saves/tests. Update all spawn sites instead.
- **Zero clippy warnings**: Always run `cargo clippy` before committing.
- **Reference manual.pdf**: For game mechanics, rules, and original Imperialism behavior.
- **When in doubt, ask**: Clarify requirements rather than making assumptions.

## Adding Systems

Register in appropriate plugin. Use run conditions and SystemSets:
```rust
app.add_systems(
    OnEnter(TurnPhase::Processing),
    my_system.in_set(ProcessingSet::Production),
);
```

**Data organization:** Per-nation → Components, Global → Resources, Player input → Messages/Events.

## Key References

- `manual.pdf` - Game mechanics and rules
- `ai-docs/ALLOCATION_DESIGN.md` - Allocation system details
- `OVERVIEW.md` - High-level game design
