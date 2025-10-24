# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. Last updated: **2025-10-20**.

**This is an economy-first, turn-based strategy game** inspired by Imperialism (1997). Built with Bevy 0.17 ECS, featuring hex-based maps, multi-nation economies, and a reservation-based resource allocation system.

## Recent Changes (Oct 2025)

- **Debug overlays**: Transport network connectivity visualization (F3), connected resource display (C key)
- **Civilian units**: Complete prospector/farmer/forester/engineer system with resource discovery
- **Prospecting system**: Hidden mineral deposits with visual markers (red X or colored squares)
- **Rescind orders**: Exclusive-world-access system for immediate component removal
- **UI patterns**: Documented Bevy 0.17 button requirements (both Button + OldButton components)
- **Plugin architecture**: `EconomyPlugin`, `MapSetupPlugin`, `CameraPlugin` encapsulate system registration
- **Resources/messages**: Moved to respective plugins (Economy/Map own their resources)
- **Map visibility**: All map visuals use `MapTilemap` marker for automatic show/hide on mode switch
- **Module structure**: `lib.rs` reduced to 76 lines (pure plugin orchestration), major modules use subdirectories
- **Allocation system**: Refactored to atomic reservations (`Vec<ReservationId>` per allocation)
- **Test organization**: Inline for small tests (<50 lines), separate `tests.rs` for large test suites
- **Import style**: All code uses explicit `crate::` paths (no `super::`)
- **Quality**: Zero clippy warnings, 119 unit + 5 integration tests passing

## Quick Reference

**Build & Run:**
```bash
cargo run              # Run game
cargo test             # Run all tests
cargo clippy           # Lint checks
```

**Debug Overlays (In-Game):**
- **F3**: Toggle transport network connectivity visualization
  - Green lines: Rails connected to your capital
  - Red lines: Disconnected rail segments
  - Labels show depot/port connectivity status
- **C**: Toggle connected resource production display
  - Shows which resources are accessible via transport network
  - Color-coded by source (improvements, ports, baseline)

**Where to find things:**
- Plugins: `src/economy/mod.rs`, `src/map/mod.rs`, `src/helpers/camera.rs`, `src/civilians/mod.rs`
- App orchestration: `src/lib.rs` (76 lines, no implementation)
- Allocation details: `ai-docs/ALLOCATION_DESIGN.md`
- Game mechanics reference: `OVERVIEW.md`

**Tech stack:**
- Engine: Bevy 0.17, `bevy_ecs_tilemap` 0.17.0-rc.1, `hexx` 0.21
- States: `AppState` (MainMenu/InGame), `GameMode` (Map/Transport/City/Market/Diplomacy)
- Turn loop: PlayerTurn → Processing → EnemyTurn

## Architecture

**Plugin-based:**
- Each subsystem has its own plugin (Economy, Map, Camera, Civilians, Diplomacy, UI)
- Plugins register systems, resources, and messages
- Plugins defined in module `mod.rs` files
- `lib.rs` only orchestrates plugins

**Three-layer separation:**
```
Input Layer (observers, events) → Logic Layer (systems, state) → Rendering Layer (visuals)
```
- Input never mutates state directly
- Logic never queries UI interaction
- Rendering never mutates game logic
- Messages (`MessageWriter`/`MessageReader`) decouple layers

**ECS patterns:**
- Per-nation data: Components (`Stockpile`, `Treasury`, `Workforce`)
- Global state: Resources (`Calendar`, `TurnSystem`, `PlayerNation`)
- Visibility control: `MapTilemap` marker on all map visuals

## Project Structure

```
src/
├── lib.rs (plugin orchestration only)
├── map/
│   ├── mod.rs (MapSetupPlugin: tilemap, provinces, terrain atlas)
│   ├── tiles.rs, terrain_gen.rs, province*.rs
│   └── rendering/ (borders, cities, transport visuals)
├── economy/
│   ├── mod.rs (EconomyPlugin: all economy systems + resources)
│   ├── production.rs, allocation*.rs, goods.rs, stockpile.rs
│   ├── transport/ (rails, depots, ports, connectivity)
│   └── workforce/ (recruitment, training, consumption)
├── civilians/ (mod.rs: CivilianPlugin)
├── helpers/camera.rs (CameraPlugin)
├── ui/ (GameUIPlugin, city/, market.rs, transport.rs, diplomacy.rs)
└── turn_system.rs (TurnSystemPlugin)
```

## Resource Allocation System

Pre-allocation model (inspired by Imperialism): Reserve during PlayerTurn, commit at turn end, consume during Processing.

```
PlayerTurn → reserve resources → adjust freely
Turn End → commit reservations → lock resources
Processing → consume → produce outputs
Next Turn → reset → start fresh
```

**Key types:**
- `Allocations`: `Vec<ReservationId>` per activity (each ID = 1 unit)
- `ResourcePool`: Atomic reserve/release/consume with rollback
- Messages: `AdjustRecruitment`, `AdjustTraining`, `AdjustProduction`

See `ai-docs/ALLOCATION_DESIGN.md` for full details.

## Code Conventions

**Imports:**
- Use explicit `crate::` paths everywhere (no `super::`)
- Group: standard library → external crates → crate modules

**Modules:**
- Complex modules → subdirectories: `economy/`, `civilians/`, `ui/city/`, `map/`
- Simple modules → single files: `treasury.rs`, `calendar.rs`
- Plugins → always in `mod.rs` (not separate files)

**Testing:**
- Small tests (<50 lines): inline `#[cfg(test)] mod tests {}`
- Large tests: separate `tests.rs` in module directory
- Import style: `use crate::module::Type;` (never `super::`)

**Map visuals:**
- Always add `MapTilemap` marker to sprites/meshes visible on map
- Enables automatic visibility control via `show_screen`/`hide_screen`

**UI Buttons (Bevy 0.17):**
- MUST use BOTH button components: `Button` (new) and `OldButton` (compatibility layer)
- Import: `use bevy::ui::widget::Button as OldButton; use bevy::ui_widgets::{Activate, Button};`
- Use `.observe()` as a **builder method**, NOT as a component in the spawn tuple
- Correct pattern:
```rust
parent
    .spawn((
        Button,
        OldButton,
        Node { padding: UiRect::all(Val::Px(8.0)), ..default() },
        BackgroundColor(NORMAL_BUTTON),
    ))
    .observe(move |_: On<Activate>, /* system parameters */| {
        // Button click handler
    })
    .with_children(|button_parent| {
        button_parent.spawn((Text::new("Label"), ...));
    });
```
- ❌ WRONG: `observe(...)` inside the spawn tuple
- ✅ CORRECT: `.observe(...)` as method call after `.spawn()`

## How to Work on This Codebase

**Adding systems:**
- Register in appropriate plugin (`EconomyPlugin`, `MapSetupPlugin`, etc.)
- Use run conditions: `in_state(AppState::InGame)`, `in_state(GameMode::Map)`, etc.
- Group related systems with `.add_systems()`

**Data organization:**
- Per-nation data → Components on nation entities
- Global state → Resources
- Player input → Messages/Events

**Creating new subsystems:**
- Consider creating a new plugin in the module's `mod.rs`
- Resources/messages owned by the plugin
- UI overlays should be fullscreen with "Back to Map" button

**Key architectural rules:**
- Input/Logic/Rendering separation via messages
- Map visuals must have `MapTilemap` marker
- Plugins own their resources and messages
- Zero clippy warnings policy

## Current Feature Status

✅ **Complete:**
- Main menu, province generation, border rendering, city rendering
- Civilian units (Engineer, Prospector, Farmer, Rancher, Forester, Miner, Driller)
- Prospecting system with hidden minerals and visual discovery markers
- Rescind orders functionality with refunds for same-turn actions
- Production system (TextileMill with 2:1 ratios)
- Allocation/reservation system
- Market (fixed prices, exclusive buy/sell orders)
- Turn system with calendar
- Transport infrastructure (rails, roads, depots, ports with connectivity)
- Map visibility system (automatic hide/show on mode switch)
- Debug overlays (transport connectivity F3, resource production C)

🔲 **TODO:**
- Link cities to provinces (show province resources)
- Market v2 (order book, uniform-price clearing)
- Diplomacy (relations, treaties)
- Transport UX (selection reset, adjacency validation)
- Test coverage (roads, production math, province generation)
