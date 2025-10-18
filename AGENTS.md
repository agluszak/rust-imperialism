# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. Last updated: **2025-10-19**.

**This is an economy-first, turn-based strategy game** inspired by Imperialism (1997). Built with Bevy 0.17 ECS, featuring hex-based maps, multi-nation economies, and a reservation-based resource allocation system.

**Recent changes** (Oct 2025):
- Plugin architecture: `EconomyPlugin`, `MapSetupPlugin`, `CameraPlugin` encapsulate system registration
- Map module reorganized: all map-related files moved to `src/map/` with subdirectory structure
- `lib.rs` simplified from 361 → 110 lines, now focused on app configuration only
- Plugins defined in respective `mod.rs` files (not separate plugin files)
- Allocation system refactored to atomic reservations (`Vec<ReservationId>` per allocation)
- Major modules use subdirectory structure: `economy/{transport,workforce}`, `ui/city`, `civilians/`, `map/{rendering,province}`
- Test organization standardized: inline for small tests, separate files for large test suites
- All imports use explicit `crate::` paths (no `super::` in tests)
- Zero clippy warnings, 74 unit tests + 3 integration tests passing

**Key architectural decisions**:
- Strict Input/Logic/Rendering separation via messages
- Per-nation economy data as Components (not global Resources)
- Resource allocation follows Imperialism's pre-allocation model (adjust during turn, commit at turn end)
- Plugin-based architecture with each major subsystem in its own plugin

## Quick Reference

**Build & Run:**
```bash
cargo run              # Run game (debug)
cargo test             # Run all tests (74 unit + 3 integration)
cargo clippy           # Lint checks
```

**Key Controls:**
- WASD: pan camera | Z/X: zoom | Space: end turn
- Left-click civilian: select | Escape: deselect all
- Sidebar: Map/Transport/City/Market/Diplomacy modes

**Where to find things:**
- Game logic: `src/economy/`, `src/civilians/`
- UI: `src/ui/city/`, `src/ui/{transport,market,diplomacy}.rs`
- Plugin registration: `src/economy/mod.rs`, `src/map/mod.rs`, `src/helpers/camera.rs`
- App setup: `src/lib.rs` (orchestrates plugins, no implementation)
- Allocation system: `src/economy/allocation*.rs`, `ALLOCATION_DESIGN.md`
- Project overview: `OVERVIEW.md` (Imperialism 1997 mechanics)

## High-level overview

- Engine: Bevy 0.17, `bevy_ecs_tilemap` 0.17.0-rc.1, `hexx` 0.21
- Core loop: PlayerTurn → Processing → EnemyTurn (timed), with a seasonal calendar that advances each turn (Spring → Summer → Autumn → Winter → next year)
- Screens & modes:
  - AppState (root): `MainMenu` (default), `InGame`
  - GameMode (sub-state under `InGame`): `Map` (default), `Transport`, `City`, `Market`, `Diplomacy`
- Map: Procedural terrain (Perlin-based)
- UI: HUD (Turn, Season/Year, Treasury), terminal log with scrollbar, sidebar mode buttons; each non-Map mode shows a fullscreen overlay with a "Back to Map" button
- Economy scaffolding: Multi-nation ECS (per-nation `Treasury` and `Stockpile` components), simple Production (TextileMill), Transport events (roads toggling), minimal Market (fixed-price buy/sell of Cloth)

Read @OVERVIEW.md to get a high-level overview of the Imperialism game.

## Current features (functional)

- Main Menu
  - Fullscreen UI with "New Game" (switches to `AppState::InGame`) and "Quit"
  - Gameplay UI spawns only when entering `InGame` (no map/HUD behind the menu)

- Map Mode
  - Tilemap is generated once and persists across mode switches
  - Camera: WASD pan, Z/X (and mouse wheel) zoom
  - **Provinces & Countries**: Terrain is divided into provinces (15-20 non-water tiles each); each province has a city and belongs to a country
  - **Borders**: Wide dual-color borders mark international boundaries; thin black borders separate provinces within a country
  - **Cities & Capitals**: Visual sprites show cities (town_small) and capitals (capital) at province centers

- City (Production) Mode
  - Full production UI with warehouse stockpile display (Wool, Cotton, Cloth)
  - Dynamic building panels showing real Building + ProductionSettings data
  - Production controls per building:
    - Input choice buttons (Use Cotton / Use Wool for textile mill)
    - +/- buttons to adjust target output (capped by capacity)
    - Shows available inputs vs needed inputs
  - Production logic (runs during Processing):
    - `Building` with `capacity` (max output per turn)
    - `ProductionSettings` with `choice` and `target_output` (persists turn-to-turn)
    - Recipe: `2× Cotton OR 2× Wool → 1× Cloth` (strict 2:1 ratio)
    - Auto-reduces target when inputs insufficient
    - Player nation starts with Textile Mill (capacity 8)

- Market Mode
  - Order-based trading: place buy/sell orders during player turn, execute during processing
  - **Exclusive orders**: Cannot buy AND sell the same resource simultaneously
    - Setting buy orders automatically clears any existing sell orders for that good
    - Setting sell orders automatically clears any existing buy orders for that good
  - Uses allocation/reservation system (like production and workforce)
  - Fixed prices for MVP (will be replaced by market clearing in future)
  - Tradable resources: Grain, Fruit, Livestock, Fish, Cotton, Wool, Timber, Coal, Iron, Gold, Gems, Oil

- Turn system & Calendar
  - Press Space to end `PlayerTurn`
  - Timed phase transitions simulate Processing and EnemyTurn
  - Calendar advances on `EnemyTurn → PlayerTurn` (season rolls, year increments after Winter)
  - HUD shows current turn and calendar

- Multi-nation economy
  - Nations are entities with: `NationId`, `Name`, `NationColor`, `Capital`, `Treasury`, `Stockpile`, (and optionally `Building`s)
  - `PlayerNation(Entity)` resource points to the active player's nation
  - Each nation controls connected provinces; game typically starts with 3-5 nations

## Project structure

```
src/
├── main.rs, lib.rs (orchestrates plugins)
├── assets.rs, bmp_loader.rs, constants.rs
│
├── map/
│   ├── mod.rs (MapSetupPlugin, tilemap creation, tile hover handlers)
│   ├── tiles.rs, terrain_gen.rs, tile_pos.rs
│   ├── province.rs, province_gen.rs, province_setup.rs
│   └── rendering/
│       ├── mod.rs
│       ├── map_visual.rs, terrain_atlas.rs
│       ├── border_rendering.rs, city_rendering.rs
│       └── transport_rendering.rs
│
├── helpers/
│   ├── camera.rs (CameraPlugin, setup, movement, centering)
│   └── picking.rs
│
├── input.rs (InputPlugin, tile click handlers)
├── turn_system.rs (TurnSystemPlugin)
│
├── civilians/
│   ├── mod.rs (CivilianPlugin)
│   ├── types.rs, commands.rs, jobs.rs
│   ├── systems.rs, engineering.rs
│   ├── rendering.rs, ui_components.rs
│   └── tests.rs
│
├── economy/
│   ├── mod.rs (EconomyPlugin, system registration)
│   ├── goods.rs, stockpile.rs, treasury.rs
│   ├── calendar.rs, nation.rs
│   ├── production.rs, technology.rs
│   ├── allocation.rs, allocation_systems/, reservation.rs
│   ├── transport/
│   │   ├── types.rs, messages.rs, validation.rs
│   │   ├── construction.rs, connectivity.rs, input.rs
│   │   └── mod.rs
│   └── workforce/
│       ├── types.rs, systems.rs
│       ├── recruitment.rs, training.rs, consumption.rs
│       └── mod.rs
│
└── ui/
    ├── mod.rs (GameUIPlugin), components.rs, setup.rs
    ├── logging.rs, input.rs, status.rs
    ├── mode.rs, menu.rs
    ├── state/
    ├── city/
    │   ├── components.rs, layout.rs
    │   ├── production.rs, workforce.rs
    │   ├── allocation_ui_unified.rs, allocation_widgets.rs
    │   ├── buildings/, dialogs/, hud/
    │   └── mod.rs
    ├── transport.rs
    ├── market.rs
    └── diplomacy.rs
```

**Key modules:**
- **map**: Tilemap setup, procedural generation, hex utilities, atlas building, provinces, borders, city rendering
- **economy**: EconomyPlugin with all economy systems, per-nation Stockpile/Treasury, production, allocation, transport, workforce
- **civilians**: CivilianPlugin, unit types, multi-turn jobs, Engineer/Prospector logic
- **helpers/camera**: CameraPlugin with setup, movement, and positioning systems
- **ui**: GameUIPlugin, HUD, terminal log, mode overlays (City/Transport/Market/Diplomacy)

## Important types

**States:**
- `AppState`: `MainMenu` | `InGame`
- `GameMode`: `Map` | `Transport` | `City` | `Market` | `Diplomacy`

**Geography:**
- `Province`, `ProvinceId`, `TileProvince`, `City` (Components)
- Functions: `generate_provinces()`, `assign_provinces_to_countries()`

**Economy:**
- Per-nation: `Stockpile`, `Treasury`, `Workforce`, `Allocations` (Components)
- Global: `Calendar` (Resource)
- Nation identity: `NationId`, `Name`, `NationColor`, `Capital` (Components), `PlayerNation` (Resource)
- Production: `Building`, `ProductionSettings` (Components), `Good` (enum)
- Allocation: `Allocations`, `ReservationId`, `ResourcePool`
- Transport: `ImprovementKind`, `Roads`, `Rails`, `Depot`, `Port`

**Civilians:**
- `Civilian`, `CivilianJob` (Components)
- `CivilianKind`: Engineer, Prospector, Miner, Farmer, Rancher, Forester, Driller, Developer

**Turn System:**
- `TurnSystem` (Resource): `current_turn`, `phase`, `end_player_turn()`, `is_player_turn()`
- Systems: `process_turn_phases`, `reset_civilian_actions`, `advance_civilian_jobs`

## Controls (runtime)

- **New Game** (Main Menu)
- **WASD**: pan camera; **Z/X**: zoom out/in; **mouse wheel**: zoom (or scroll terminal when hovered)
- **Left-click civilian**: select/deselect unit (yellow tint when selected)
- **Escape**: deselect all civilians
- **Engineer orders** (when selected): buttons to Build Depot or Build Port
- **Transport Mode**: left-click two tiles to toggle a road (charges/credits $10)
- **Market Mode**: use +/- buttons to place buy OR sell orders (mutually exclusive per resource)
- **Space**: end PlayerTurn
- **Sidebar**: Map/Transport/City/Market/Diplomacy; each overlay includes a "Back to Map" button

## Testing

**Test Organization (Two Simple Patterns):**

1. **Small tests (< 50 lines)**: Inline `#[cfg(test)] mod tests { }` at end of module file
   ```rust
   // In module_name.rs
   #[cfg(test)]
   mod tests {
       use crate::module::Type;
       #[test]
       fn test_something() {}
   }
   ```
   - Example: `src/ui/state.rs`

2. **Large tests (> 50 lines)**: Separate `tests.rs` in module subdirectory
   ```rust
   // In module_name/mod.rs (or single file → convert to directory)
   #[cfg(test)]
   mod tests;  // Rust automatically finds tests.rs
   ```
   - Examples: `src/civilians/tests.rs`, `src/turn_system/tests.rs`, `src/economy/allocation_systems/tests.rs`
   - **Important**: If module is a single file with large tests, convert it to a directory first
     (`module.rs` → `module/mod.rs` + `module/tests.rs`)

**Import Style:**
- All test imports use explicit `crate::` paths (never `super::` or `use super::*`)
- Example: `use crate::turn_system::{TurnPhase, TurnSystem};`
- Never use `#[path = "..."]` attribute for test files

**Coverage:**
- Unit tests: economy (goods, stockpile, allocation, workforce), turn system, UI state, civilians
- Integration tests: turn transitions, tile properties, UI formatting, hex utilities
- Run: `cargo test` (74 unit tests + 3 integration tests, all passing)

## Code Style & Conventions

**Imports:**
- Use explicit `crate::` paths in all code (production and tests)
- Avoid `super::` and wildcard imports (`use super::*`)
- Group imports: standard library → external crates → crate modules

**Module Organization:**
- Subdirectories for complex modules: `economy/`, `civilians/`, `ui/city/`, `map/`
- Single files for simple modules: `treasury.rs`, `calendar.rs`
- Public API via re-exports in `mod.rs`: `pub use submodule::Type;`
- **Plugins always in `mod.rs`**: Plugin implementations belong in the module's `mod.rs`, not separate files

**ECS Architecture:**
- Per-nation data: Components on nation entities (`Stockpile`, `Treasury`, `Workforce`)
- Global game state: Resources (`Calendar`, `TurnSystem`, `PlayerNation`)
- Player input: Events/Messages (`PlaceImprovement`, `AdjustProduction`)
- Three-layer separation: Input → Logic → Rendering (see Architecture section)

**Plugin Pattern:**
- Each major subsystem has its own plugin (e.g., `EconomyPlugin`, `MapSetupPlugin`, `CameraPlugin`)
- Plugins defined in respective `mod.rs` files
- `lib.rs` is minimal, just orchestrates plugins
- System registration grouped logically within plugins

**Testing:**
- Follow standardized test organization patterns (see Testing section)
- Run `cargo test` and `cargo clippy` before committing
- Aim for zero warnings

## How to work on this codebase

- Prefer events for player commands (`PlaceImprovement`, future `AdjustWorkers`, `PlaceOrder`, etc.)
- Treat per-nation economy data as Components on nation entities; avoid global resources for these
- Keep truly global concepts as Resources: app/game states, `Calendar`, terminal log
- UI overlays for modes should be full-screen and include a "Back to Map" button to avoid input occlusion issues
- When adding new systems:
  - Register them in the appropriate plugin (`EconomyPlugin`, `MapSetupPlugin`, etc.)
  - Use appropriate run conditions: `in_state(AppState::InGame)`, `in_state(GameMode::Transport)`, etc.
  - Group related systems together with `.add_systems()`
- When creating a new major subsystem, consider creating a new plugin in the module's `mod.rs`

## Architecture

**Three-layer separation:** Input → Logic → Rendering

```
User Input → Input Handler → Message
                               ↓
                          Logic System → Game State (Components/Resources)
                                             ↓
                                        Rendering System → Visuals (Sprites/UI)
```

**Principles:**
- **Input Layer**: Reads `Interaction`, emits messages (never mutates state)
- **Logic Layer**: Reads messages, mutates Components/Resources (never queries `Interaction`)
- **Rendering Layer**: Reads state, spawns/updates visuals (never mutates game logic)
- Messages (`MessageWriter`/`MessageReader`) decouple input from logic
- Layers can coexist in same file but remain conceptually separate

**Plugin Architecture:**
- Each major subsystem (economy, map, camera, civilians, diplomacy, UI) has its own plugin
- Plugins encapsulate system registration, keeping `lib.rs` clean
- Plugins defined in their module's `mod.rs` for easy discovery
- `lib.rs` acts as orchestrator: configures app, registers plugins, no implementation details

## Resource Allocation System

**Pre-allocation model** (inspired by Imperialism): Resources are reserved during PlayerTurn, committed at turn end, consumed during Processing.

```
PlayerTurn → reserve resources → adjust freely
Turn End → commit reservations → lock resources
Processing → consume → produce outputs
Next Turn → reset → start fresh
```

**Key types:**
- `Allocations` (Component): `Vec<ReservationId>` per production/recruitment/training (each ID = 1 unit)
- `ResourcePool`: Atomic reserve/release/consume operations with rollback support
- Messages: `AdjustRecruitment`, `AdjustTraining`, `AdjustProduction`

**Systems:**
- `apply_*_adjustments` - reserve resources unit-by-unit during PlayerTurn
- `finalize_allocations` - consume reservations at turn end
- `reset_allocations` - release all reservations at turn start

**See [ALLOCATION_DESIGN.md](ai-docs/ALLOCATION_DESIGN.md) for full details on implementation, UI patterns, and resource reservation mechanics.**

## Roadmap (short)

1) **Province & City Interaction** ✅ DONE
- ✅ Province generation (15-20 tiles, flood-fill)
- ✅ Province assignment to countries (connected groups)
- ✅ Border rendering (international dual-color, provincial black)
- ✅ City and capital sprite rendering

2) **Transport visuals & UX**
- ✅ Depot and port sprite rendering
- ✅ Render railway overlay (lines between hex centers)
- 🔲 Reset selection on mode exit; adjacency validation with user feedback

3) **City data-binding** ✅ DONE
- ✅ Populate City screen from real `Building`s and `Stockpile`
- ✅ Add production +/- controls (event-driven) with input choice buttons
- ✅ Show warehouse stockpile and available inputs vs needed
- 🔲 Link cities to provinces and show province resources

4) **Market v2**
- 🔲 Replace fixed buttons with a simple order book and uniform-price clearing
- 🔲 Track/display last prices and open orders

5) **Diplomacy stub**
- 🔲 Minimal `Relation` per nation pair and two actions (Improve Relations, Trade Treaty)

6) **Tests & cleanup**
- ✅ Code organization and plugin architecture
- 🔲 Add coverage for roads toggling, production math, market clearing
- 🔲 Add tests for province generation and assignment
