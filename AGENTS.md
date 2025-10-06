# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. It reflects the repository as of 2025-10-07.

If you remember older arcade/RPG features (hero, monsters, combat, health, pathfinding) — those were removed. This is now an economy-first, turn-based strategy prototype inspired by Imperialism (1997).

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

- City (Production) Mode (MVP)
  - UI overlay stub shows a City Overview with demo rows
  - Production logic exists in the world:
    - `Building::textile_mill(workers)` runs during Processing
    - Recipe: `1x Wool + 1x Cotton → 1x Cloth` per worker, limited by inputs
    - Attached to the Player nation at game start (workers = 4)

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

## Project structure (actual)

```
src/
├── main.rs               # App wiring, camera, state setup, map generation, systems registration
├── lib.rs                # Library entry point with plugin registration
├── assets.rs             # Asset path mapping (terrain, units, cities, transport)
├── bmp_loader.rs         # Custom BMP loader with transparency handling
├── constants.rs          # Tunable constants (map size, tile size, seeds)
├── tiles.rs              # Tile categories and properties + texture index mapping
├── terrain_gen.rs        # Perlin-based procedural terrain classifier
├── terrain_atlas.rs      # Terrain texture atlas building
├── tile_pos.rs           # Hex ↔ tile utilities and world-position helpers
├── input.rs              # Pointer click routing (terrain edit, transport selection)
├── turn_system.rs        # Turn phases, timers, calendar advancement
├── province.rs           # Province, City, ProvinceId, TileProvince components
├── province_gen.rs       # Province generation via flood-fill (15-20 tiles each)
├── province_setup.rs     # Province assignment to countries (connected groups)
├── border_rendering.rs   # Renders international (dual-color) and provincial borders
├── city_rendering.rs     # Renders city and capital sprites
├── civilians.rs          # Civilian units (Engineer, Prospector, etc.) with visual rendering
├── transport_rendering.rs # Visual rendering for roads, rails, depots, ports
├── economy/
│   ├── goods.rs          # `Good` enum (Wool, Cotton, Cloth)
│   ├── stockpile.rs      # `Stockpile` (Component) with helpers
│   ├── treasury.rs       # `Treasury` (Component)
│   ├── calendar.rs       # Global `Calendar` (Resource)
│   ├── nation.rs         # `NationId`, `Name`, `NationColor`, `Capital` (Components), `PlayerNation` (Resource)
│   ├── transport.rs      # `ImprovementKind`, `PlaceImprovement`, `Roads`, `Rails`, `apply_improvements`
│   └── production.rs     # `Building`, `BuildingKind`, `run_production`
└── ui/
    ├── mod.rs            # UI plugin (messages, state collection, scheduling)
    ├── components.rs     # UI marker components (HUD/terminal/roots)
    ├── setup.rs          # HUD/terminal/sidebar (spawned on entering InGame)
    ├── logging.rs        # TerminalLog resource + events and rendering
    ├── input.rs          # Terminal scroll + clamping
    ├── status.rs         # HUD updaters (Turn/Calendar/Treasury)
    ├── state/            # Centralized `UIState` + tests
    ├── mode.rs           # `GameMode` SubState + button handlers
    ├── menu.rs           # `AppState` (MainMenu/InGame) + main menu UI
    ├── city.rs           # City overlay (Back to Map button)
    ├── transport.rs      # Transport overlay + click-to-edge tool
    ├── market.rs         # Market overlay + fixed buy/sell
    └── diplomacy.rs      # Diplomacy overlay (stub)
```

## Important types and systems

- States
  - `AppState` (States): `MainMenu` | `InGame`
  - `GameMode` (SubStates; source = `AppState::InGame`): `Map` | `Transport` | `City` | `Market` | `Diplomacy`

- Geography & Political
  - `Province` (Component) — owns multiple tiles, has a city, belongs to a nation
  - `ProvinceId` (Component) — stable identifier
  - `TileProvince` (Component on tiles) — links each tile to its province
  - `City` (Component) — marks city/capital entity with `province: ProvinceId, is_capital: bool`
  - Province generation: `generate_provinces()` creates 15-20 tile groups via flood-fill
  - Province assignment: `assign_provinces_to_countries()` groups connected provinces per nation

- Economy
  - `Good` (Wool, Cotton, Cloth)
  - `Stockpile` (Component, per nation) with `add`, `get`, `take_up_to`, `has_at_least`
  - `Treasury` (Component, per nation)
  - `Calendar` (Resource) — world time; `display()` returns e.g., "Spring, 1815"
  - `NationId`, `Name`, `NationColor`, `Capital` (Components); `PlayerNation(Entity)` (Resource)
  - Production: `Building`, `BuildingKind::TextileMill(u8 workers)`; system: `run_production`
  - Transport: `ImprovementKind::Road|Rail|Depot|Port`, `PlaceImprovement { a, b, kind }`; `Roads`, `Rails` (Resources); system: `apply_improvements`

- Civilians
  - `Civilian` (Component) with `kind: CivilianKind`, `position: TilePos`, `owner: Entity`, `selected: bool`, `has_moved: bool`
  - `CivilianKind` enum: Engineer, Prospector, Miner, Farmer, Rancher, Forester, Driller, Developer
  - `CivilianJob` (Component) — multi-turn jobs with `turns_remaining`
  - Engineers can build rails, depots, ports (with 2-3 turn construction times)

- Input/UI
  - Pointer click handler: left-click civilians → select
  - HUD updaters: `update_turn_display`, `update_calendar_display`, `update_treasury_display`
  - Terminal: `TerminalLogEvent` → `TerminalLog` → text rendering with scrolling
  - Engineer orders panel appears when Engineer selected

- Rendering
  - `render_borders()` — draws international borders (dual-color, offset) and provincial borders (thin black)
  - `render_city_visuals()` — spawns sprites for cities and capitals at z=2.0
  - `render_civilian_visuals()` — spawns sprites for civilians at z=3.0, tints yellow when selected
  - `render_transport_improvements()` — draws roads, rails, depots, ports

- Turn system
  - `TurnSystem` resource with `current_turn`, `phase`, helpers (`end_player_turn`, `is_player_turn`)
  - `process_turn_phases` handles timers and advances the `Calendar` on each full turn
  - `reset_civilian_actions` and `advance_civilian_jobs` run at start of player turn

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

3) **City data-binding**
- 🔲 Populate City screen from real `Building`s and `Stockpile`
- 🔲 Add worker +/- controls (event-driven) and utilization bars
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

## Build & run

```bash
# Build
cargo build

# Run the game (debug)
cargo run

# Run tests
cargo test

# Lints (may auto-fix some issues)
cargo clippy --fix --allow-dirty --allow-staged
```
