# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. It reflects the repository as of 2025-10-05.

If you remember older arcade/RPG features (hero, monsters, combat, health, pathfinding) — those were removed. This is now an economy-first, turn-based strategy prototype inspired by Imperialism (1997).

## High-level overview

- Engine: Bevy 0.17, `bevy_ecs_tilemap` 0.17.0-rc.1, `hexx` 0.21
- Core loop: PlayerTurn → Processing → EnemyTurn (timed), with a seasonal calendar that advances each turn (Spring → Summer → Autumn → Winter → next year)
- Screens & modes:
  - AppState (root): `MainMenu` (default), `InGame`
  - GameMode (sub-state under `InGame`): `Map` (default), `Transport`, `City`, `Market`, `Diplomacy`
- Map: Procedural terrain (Perlin-based), editable in dev via right-click cycling
- UI: HUD (Turn, Season/Year, Treasury), terminal log with scrollbar, sidebar mode buttons; each non-Map mode shows a fullscreen overlay with a “Back to Map” button
- Economy scaffolding: Multi-nation ECS (per-nation `Treasury` and `Stockpile` components), simple Production (TextileMill), Transport events (roads toggling), minimal Market (fixed-price buy/sell of Cloth)

Read @OVERVIEW.md to get a high-level overview of the Imperialism game.

## Current features (functional)

- Main Menu
  - Fullscreen UI with “New Game” (switches to `AppState::InGame`) and “Quit”
  - Gameplay UI spawns only when entering `InGame` (no map/HUD behind the menu)

- Map Mode
  - Tilemap is generated once and persists across mode switches
  - Right-click any tile to cycle terrain type (for prototyping)
  - Camera: WASD pan, Z/X (and mouse wheel) zoom

- Transport Mode (MVP)
  - Click two tiles to toggle a road edge between them (no visual overlay yet)
  - Treasury cost: $10 per road placement/removal (applied to the Player nation)
  - Events: `TransportSelectTile` (UI) → `PlaceImprovement { Road }` (domain)
  - Data: `Roads` resource stores undirected edges as ordered tile pairs

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
  - Nations are entities with: `NationId`, `Name`, `Treasury`, `Stockpile`, (and optionally `Building`s)
  - `PlayerNation(Entity)` resource points to the active player’s nation

## Project structure (actual)

```
src/
├── main.rs            # App wiring, camera, state setup, map generation, systems registration
├── constants.rs       # Tunable constants (map size, tile size, seeds)
├── tiles.rs           # Tile categories and properties + texture index mapping
├── terrain_gen.rs     # Perlin-based procedural terrain classifier
├── tile_pos.rs        # Hex ↔ tile utilities and world-position helpers
├── input.rs           # Pointer click routing (terrain edit, transport selection)
├── turn_system.rs     # Turn phases, timers, calendar advancement
├── economy/
│   ├── goods.rs       # `Good` enum (Wool, Cotton, Cloth)
│   ├── stockpile.rs   # `Stockpile` (Component) with helpers
│   ├── treasury.rs    # `Treasury` (Component)
│   ├── calendar.rs    # Global `Calendar` (Resource)
│   ├── nation.rs      # `NationId`, `Name` (Components), `PlayerNation` (Resource)
│   ├── transport.rs   # `ImprovementKind`, `PlaceImprovement`, `Roads`, `apply_improvements`
│   └── production.rs  # `Building`, `BuildingKind`, `run_production`
└── ui/
    ├── mod.rs         # UI plugin (messages, state collection, scheduling)
    ├── components.rs  # UI marker components (HUD/terminal/roots)
    ├── setup.rs       # HUD/terminal/sidebar (spawned on entering InGame)
    ├── logging.rs     # TerminalLog resource + events and rendering
    ├── input.rs       # Terminal scroll + clamping
    ├── status.rs      # HUD updaters (Turn/Calendar/Treasury)
    ├── state/         # Centralized `UIState` + tests
    ├── mode.rs        # `GameMode` SubState + button handlers
    ├── menu.rs        # `AppState` (MainMenu/InGame) + main menu UI
    ├── city.rs        # City overlay (Back to Map button)
    ├── transport.rs   # Transport overlay + click-to-edge tool
    ├── market.rs      # Market overlay + fixed buy/sell
    └── diplomacy.rs   # Diplomacy overlay (stub)
```

## Important types and systems

- States
  - `AppState` (States): `MainMenu` | `InGame`
  - `GameMode` (SubStates; source = `AppState::InGame`): `Map` | `Transport` | `City` | `Market` | `Diplomacy`

- Economy
  - `Good` (Wool, Cotton, Cloth)
  - `Stockpile` (Component, per nation) with `add`, `get`, `take_up_to`, `has_at_least`
  - `Treasury` (Component, per nation)
  - `Calendar` (Resource) — world time; `display()` returns e.g., "Spring, 1815"
  - `NationId`, `Name` (Components); `PlayerNation(Entity)` (Resource)
  - Production: `Building`, `BuildingKind::TextileMill(u8 workers)`; system: `run_production`
  - Transport: `ImprovementKind::Road`, `PlaceImprovement { a, b, kind }`; `Roads(HashSet<(TilePos, TilePos)>)`; system: `apply_improvements`

- Input/UI
  - Pointer click handler: right-click → terrain cycle; left-click in `Transport` → `TransportSelectTile`
  - HUD updaters: `update_turn_display`, `update_calendar_display`, `update_treasury_display`
  - Terminal: `TerminalLogEvent` → `TerminalLog` → text rendering with scrolling

- Turn system
  - `TurnSystem` resource with `current_turn`, `phase`, helpers (`end_player_turn`, `is_player_turn`)
  - `process_turn_phases` handles timers and advances the `Calendar` on each full turn

## Controls (runtime)

- New Game (Main Menu)
- WASD: pan camera; Z/X: zoom out/in; mouse wheel: zoom (or scroll terminal when hovered)
- Right-click tile: cycle terrain type (prototype tool)
- Transport Mode: left-click two tiles to toggle a road (charges/credits $10)
- Market Mode: click buy/sell buttons to adjust Cloth and money (fixed $50)
- Space: end PlayerTurn
- Sidebar: Map/Transport/City/Market/Diplomacy; each overlay includes a "Back to Map" button

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

1) Transport visuals & UX
- Render roads overlay (lines between hex centers)
- Reset selection on mode exit; adjacency validation with user feedback

2) City data-binding
- Populate City screen from real `Building`s and `Stockpile`
- Add worker +/- controls (event-driven) and utilization bars

3) Market v2
- Replace fixed buttons with a simple order book and uniform-price clearing
- Track/display last prices and open orders

4) Diplomacy stub
- Minimal `Relation` per nation pair and two actions (Improve Relations, Trade Treaty)

5) Map selection & inspector
- Left-click selects tile/province; inspector panel shows terrain, deposits, owner; shortcuts to city/transport

6) Tests & cleanup
- Add coverage for roads toggling, production math, market clearing; clear warnings

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
