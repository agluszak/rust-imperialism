# AGENTS.md

This document is the single source of truth for contributors (human or AI) to understand the current state of the project and how to work on it. Last updated: **2025-11-27**.

**This is an economy-first, turn-based strategy game** inspired by Imperialism (1997). Built with Bevy 0.17 ECS, featuring hex-based maps, multi-nation economies, and a reservation-based resource allocation system.

## Recent Changes (Oct-Nov 2025)

- **Turn system refactor**: Complete rewrite using Bevy States for phase management
  - `TurnPhase` is now a proper Bevy State with `OnEnter`/`OnExit` schedules
  - SystemSets guarantee execution order: `PlayerTurnSet`, `ProcessingSet`, `EnemyTurnSet`
  - Auto-transitions: Processing→EnemyTurn→PlayerTurn happen automatically
  - No more `resource_changed::<TurnSystem>` pattern (fired multiple times)
  - Legacy `TurnSystem` resource kept for backward compatibility (synced from state)
- **Transport to stockpile**: Connected resources now properly collected into nation stockpiles
- **AI opponents**: Integrated big-brain behavior system with economy planning and civilian management
- **Save/load system**: Full game persistence using moonshine-save with component serialization
- **Port fish production**: Connected ports yield 2 fish (bonus from transport connectivity)
- **Market improvements**: Refactored pricing model, order matching across turn phases
- **Orders queue**: Centralized command queuing system for deferred execution
- **Debug overlays**: Transport network connectivity visualization (F3), connected resource display (C key)
- **Civilian units**: Complete prospector/farmer/forester/engineer system with resource discovery
- **Prospecting system**: Hidden mineral deposits with visual markers (red X or colored squares)
- **Rescind orders**: Exclusive-world-access system for immediate component removal
- **UI patterns**: Documented Bevy 0.17 button requirements (both Button + OldButton components)
- **Plugin architecture**: `EconomyPlugin`, `MapSetupPlugin`, `CameraPlugin`, `AiBehaviorPlugin` encapsulate system registration
- **Resources/messages**: Moved to respective plugins (Economy/Map/AI own their resources)
- **Map visibility**: All map visuals use `MapTilemap` marker for automatic show/hide on mode switch
- **Module structure**: `lib.rs` reduced to pure plugin orchestration, major modules use subdirectories
- **Allocation system**: Refactored to atomic reservations (`Vec<ReservationId>` per allocation)
- **Test organization**: Inline for small tests (<50 lines), separate `tests.rs` for large test suites
- **Import style**: All code uses explicit `crate::` paths (no `super::`)
- **Quality**: Zero clippy warnings policy, comprehensive test coverage

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
- Plugins: `src/economy/mod.rs`, `src/map/mod.rs`, `src/helpers/camera.rs`, `src/civilians/mod.rs`, `src/ai/mod.rs`
- App orchestration: `src/lib.rs` (pure plugin orchestration, no implementation)
- Allocation details: `ai-docs/ALLOCATION_DESIGN.md`
- Game mechanics reference: `OVERVIEW.md` and `manual.pdf`
- Save system: `src/save.rs`

**Tech stack:**
- Engine: Bevy 0.17, `bevy_ecs_tilemap` 0.17, `hexx` 0.22
- AI: `big-brain` (utility-based behavior trees)
- Save: `moonshine-save` and `moonshine-kind` for serialization
- States: `AppState` (MainMenu/InGame), `GameMode` (Map/Transport/City/Market/Diplomacy), `TurnPhase` (PlayerTurn/Processing/EnemyTurn)
- Turn loop: PlayerTurn → Processing → EnemyTurn (auto-transitions)

## Architecture

**Plugin-based:**
- Each subsystem has its own plugin (Economy, Map, Camera, Civilians, Diplomacy, AI, UI, Save)
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
- Global state: Resources (`Calendar`, `TurnCounter`, `PlayerNation`)
- Turn phase: Bevy State (`TurnPhase`) with `OnEnter` schedules
- Visibility control: `MapTilemap` marker on all map visuals

**Turn System Architecture:**
```
TurnPhase (Bevy State)        SystemSets (execution order)
─────────────────────         ──────────────────────────────
PlayerTurn                    PlayerTurnSet: Collection → Maintenance → Market → Reset → Ui
    ↓ (Space key)
Processing                    ProcessingSet: Finalize → Production → Conversion
    ↓ (auto)
EnemyTurn                     EnemyTurnSet: Setup → Analysis → Decisions → Actions → Orders
    ↓ (auto)
PlayerTurn (next turn)
```
- Use `OnEnter(TurnPhase::*)` for systems that run once per phase entry
- Use `in_state(TurnPhase::*)` for continuous systems during a phase
- Legacy `TurnSystem` resource is synced for backward compatibility

## Project Structure

```
src/
├── lib.rs (plugin orchestration only)
├── map/
│   ├── mod.rs (MapSetupPlugin: tilemap, provinces, terrain atlas)
│   ├── tiles.rs, terrain_gen.rs, province*.rs, prospecting.rs
│   └── rendering/ (borders, cities, transport visuals, debug overlays)
├── economy/
│   ├── mod.rs (EconomyPlugin: all economy systems + resources)
│   ├── production.rs, allocation*.rs, goods.rs, stockpile.rs
│   ├── transport/ (rails, depots, ports, connectivity)
│   ├── workforce/ (recruitment, training, consumption)
│   └── market.rs, trade.rs
├── civilians/ (mod.rs: CivilianPlugin)
├── ai/
│   ├── mod.rs (AiSupportPlugin, exports)
│   ├── behavior.rs (AiBehaviorPlugin: big-brain integration)
│   ├── trade.rs (AiEconomyPlugin: market decisions)
│   ├── context.rs (turn context for AI decisions)
│   └── markers.rs (AiNation, AiControlledCivilian)
├── diplomacy/ (mod.rs: DiplomacyPlugin)
├── orders/ (mod.rs: OrdersQueue for command queueing)
├── save.rs (GameSavePlugin: moonshine-save integration)
├── helpers/ (camera.rs: CameraPlugin, picking.rs)
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

## Development Guidelines

**Core principles:**
- **Write testable code**: Structure code to be easily tested, use dependency injection, keep functions pure when possible
- **Rely on the manual**: Reference `manual.pdf` for game mechanics, rules, and original Imperialism (1997) behavior
- **When in doubt, ask**: Use the AskUserQuestion tool to clarify requirements rather than making assumptions

## How to Work on This Codebase

**Adding systems:**
- Register in appropriate plugin (`EconomyPlugin`, `MapSetupPlugin`, etc.)
- Use run conditions: `in_state(AppState::InGame)`, `in_state(GameMode::Map)`, etc.
- For turn-based systems: use `OnEnter(TurnPhase::*)` with appropriate SystemSet
- Group related systems with `.add_systems()`

**Turn-based systems pattern:**
```rust
// System runs once when entering Processing phase
app.add_systems(
    OnEnter(TurnPhase::Processing),
    my_system.in_set(ProcessingSet::Production),
);

// System runs continuously during EnemyTurn
app.add_systems(
    PreUpdate,
    my_ai_system
        .run_if(in_state(AppState::InGame))
        .run_if(in_state(TurnPhase::EnemyTurn)),
);
```

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
- AI opponents with big-brain behavior system (rail planning, civilian hiring, economy planning)
- Save/load system with full game state persistence
- Prospecting system with hidden minerals and visual discovery markers
- Rescind orders functionality with refunds for same-turn actions
- Production system (TextileMill with 2:1 ratios)
- Allocation/reservation system
- Market with pricing model and cross-turn order matching
- Port fish production (2 fish per connected port)
- Turn system with calendar and state-based phase management
- Transport infrastructure (rails, roads, depots, ports with connectivity)
- Map visibility system (automatic hide/show on mode switch)
- Debug overlays (transport connectivity F3, resource production C)
- Orders queue for centralized command management

🔲 **TODO:**
- Link cities to provinces (show province resources in city view)
- Market v2 (order book UI, uniform-price clearing algorithm)
- Diplomacy (relations tracking, treaty system)
- Transport UX improvements (selection reset, adjacency validation)
- Test coverage expansion (roads, production math, province generation)
