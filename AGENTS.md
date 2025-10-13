# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. Last updated: **2025-10-13**.

**This is an economy-first, turn-based strategy game** inspired by Imperialism (1997). Built with Bevy 0.17 ECS, featuring hex-based maps, multi-nation economies, and a reservation-based resource allocation system.

**Recent changes** (Oct 2025):
- Allocation system refactored to atomic reservations (`Vec<ReservationId>` per allocation)
- Major modules now use subdirectory structure: `economy/{transport,workforce}`, `ui/city`, `civilians/`
- Dead code cleanup: ~220 lines removed, all `_v2` suffixes eliminated

**Key architectural decisions**:
- Strict Input/Logic/Rendering separation via messages
- Per-nation economy data as Components (not global Resources)
- Resource allocation follows Imperialism's pre-allocation model (adjust during turn, commit at turn end)

## Quick Reference

**Build & Run:**
```bash
cargo run              # Run game (debug)
cargo test             # Run all tests
cargo clippy           # Lint checks
```

**Key Controls:**
- WASD: pan camera | Z/X: zoom | Space: end turn
- Left-click civilian: select | Escape: deselect all
- Sidebar: Map/Transport/City/Market/Diplomacy modes

**Where to find things:**
- Game logic: `src/economy/`, `src/civilians/`
- UI: `src/ui/city/`, `src/ui/{transport,market,diplomacy}.rs`
- Systems registration: `src/lib.rs`
- Allocation system: `src/economy/allocation*.rs`, `ALLOCATION_DESIGN.md`
- Project overview: `OVERVIEW.md` (Imperialism 1997 mechanics)

## High-level overview

- Engine: Bevy 0.17, `bevy_ecs_tilemap` 0.17.0-rc.1, `hexx` 0.21
- Core loop: PlayerTurn → Processing → EnemyTurn (timed), with a seasonal calendar that advances each turn (Spring → Summer → Autumn → Winter → next year)
- Screens & modes:
  - AppState (root): `MainMenu` (default), `InGame`
  - GameMode (sub-state under `InGame`): `Map` (default), `Transport`, `City`, `Market`, `Diplomacy`
- Map: Procedural terrain (Perlin-based)
- UI: HUD (Turn, Season/Year, Treasury), terminal log with scrollbar, sidebar mode buttons; each non-Map mode shows a fullscreen overlay with a “Back to Map” button
- Economy scaffolding: Multi-nation ECS (per-nation `Treasury` and `Stockpile` components), simple Production (TextileMill), Transport events (roads toggling), minimal Market (fixed-price buy/sell of Cloth)

Read @OVERVIEW.md to get a high-level overview of the Imperialism game.

## Current features (functional)

- Main Menu
  - Fullscreen UI with “New Game” (switches to `AppState::InGame`) and “Quit”
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

- Market Mode (MVP)
  - Two buttons: “Buy 1 Cloth ($50)”, “Sell 1 Cloth ($50)” (fixed price)
  - Applies to the Player nation’s `Treasury` and `Stockpile`

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
├── main.rs, lib.rs
├── assets.rs, bmp_loader.rs, constants.rs
│
├── tiles.rs, terrain_gen.rs, terrain_atlas.rs, tile_pos.rs
├── province.rs, province_gen.rs, province_setup.rs
├── border_rendering.rs, city_rendering.rs, transport_rendering.rs
│
├── input.rs
├── turn_system.rs
│
├── civilians/
│   ├── types.rs, commands.rs, jobs.rs
│   ├── systems.rs, engineering.rs
│   ├── rendering.rs, ui_components.rs
│   └── tests.rs
│
├── economy/
│   ├── goods.rs, stockpile.rs, treasury.rs
│   ├── calendar.rs, nation.rs
│   ├── production.rs, technology.rs
│   ├── allocation.rs, allocation_systems.rs, reservation.rs
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
    ├── mod.rs, components.rs, setup.rs
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
- **terrain/tiles**: Procedural generation, hex utilities, atlas building
- **provinces**: Province generation (flood-fill), assignment to countries, border rendering
- **civilians**: Unit types, multi-turn jobs, Engineer/Prospector logic
- **economy**: Goods, per-nation Stockpile/Treasury, production, allocation system, transport network, workforce
- **ui**: HUD, terminal log, mode overlays (City/Transport/Market/Diplomacy)

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
- **Market Mode**: click buy/sell buttons to adjust Cloth and money (fixed $50)
- **Space**: end PlayerTurn
- **Sidebar**: Map/Transport/City/Market/Diplomacy; each overlay includes a "Back to Map" button

## Testing

- Unit tests included for `goods`, `stockpile`, `calendar`, UI state, and turn system
- Integration tests cover turn transitions, tile properties, UI state formatting, and hex utilities
- `cargo test` currently passes (warnings may appear, but no failures)

## How to work on this codebase

- Prefer events for player commands (`PlaceImprovement`, future `AdjustWorkers`, `PlaceOrder`, etc.)
- Treat per-nation economy data as Components on nation entities; avoid global resources for these
- Keep truly global concepts as Resources: app/game states, `Calendar`, terminal log
- UI overlays for modes should be full-screen and include a "Back to Map" button to avoid input occlusion issues
- When adding new systems, register them with appropriate run conditions:
  - Example: economy systems run in `Update` while `in_state(AppState::InGame)`
  - Mode-specific UI logic can run with `run_if(in_state(GameMode::Transport))`, etc.

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

**See [ALLOCATION_DESIGN.md](ALLOCATION_DESIGN.md) for full details on implementation, UI patterns, and resource reservation mechanics.**

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
- 🔲 Add coverage for roads toggling, production math, market clearing
- 🔲 Add tests for province generation and assignment
- 🔲 Clear warnings (HexExt false positive)
