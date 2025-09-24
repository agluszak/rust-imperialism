# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

This is a Rust-based hexagonal tile-based game called "rust-imperialism" built with the Bevy game engine. The project appears to be an early-stage imperialism/strategy game with hexagonal tiles that can be clicked to cycle through different terrain types.

## Key Dependencies

- **Bevy 0.16.1**: Main game engine with dynamic linking, dev tools, and mesh picking backend
- **hexx 0.21**: Hexagonal grid algorithms and utilities with Bevy integration
- **bevy_ecs_tilemap 0.16**: Tilemap rendering system for Bevy
- **rand 0.9**: Random number generation

## Common Commands

```bash
# Build the project
cargo build

# Run the game
cargo run

# Check for compilation errors without building
cargo clippy

# Run in release mode (optimized)
cargo run --release

# Build documentation
cargo doc

# Run tests
cargo test

# Fix code formatting and simple warnings
cargo clippy --fix --allow-dirty --allow-stage
```

## Project Structure

```
src/
├── main.rs           # Main game loop and tilemap setup
├── tiles.rs          # Tile system with extensive terrain/building types
├── turn_system.rs    # Turn-based gameplay system
├── hero.rs           # Hero units with movement and selection
├── monster.rs        # Monster AI system with turn-based spawning
├── health.rs         # Health and combat system
├── combat.rs         # Combat system implementation
├── input.rs          # Input handling system
├── tile_pos.rs       # Tile position utilities for hexagonal grid
├── pathfinding.rs    # A* pathfinding for hexagonal grids
├── ui/               # Game UI module (terminal, components, status, scrollbar)
└── helpers/
    ├── mod.rs        # Module declarations
    ├── camera.rs     # Camera movement and zoom controls
    └── picking.rs    # Tile click detection backend
```

## Code Architecture

### Tile System (`tiles.rs`)
- Comprehensive tile type system with `TileType` component
- Multiple categories: Terrain, Military, Resource, Building, UI
- Each tile has properties: movement cost, defense bonus, resource yield, population capacity
- Texture indices mapped to colored_packed.png tileset

### Turn System (`turn_system.rs`)
- Turn-based gameplay with `TurnSystem` resource
- Three phases: PlayerTurn, Processing, EnemyTurn
- Space key to end player turn
- Action Points refresh on phase change: Heroes at start of PlayerTurn, Monsters at start of EnemyTurn

### Hero System (`hero.rs`)
- `Hero` component with name, action points, selection state, and kill tracking
- Pathfinding with smooth animation via `MovementAnimation` (Smart movement)
- Heroes spawn as blue squares on the tilemap
- Selection tracked in state and reflected in HUD
- Action Points consumed based on terrain cost for movement
- Manual attack system that costs 1 AP — click on a monster while adjacent to attack
- Default AP: 6

### Monster System (`monster.rs`)
- `Monster` component with name, sight range, behavior types, and spawn turn tracking
- AI behaviors: Aggressive (attacks on sight), Defensive (attacks when close), Fleeing (retreats when low HP)
- Turn-based AI decisions made only during EnemyTurn phase
- Uses Simple movement (chooses best neighboring step toward target)
- Movement and attacks consume Action Points; default AP: 4; refreshed at start of EnemyTurn
- Smooth animation for movement with logical position updates
- Spawns every 3 turns with different monster types (Goblin, Orc, Skeleton)

### Combat System (`combat.rs`)
- Mixed combat system: Monsters attack via AI during EnemyTurn; Hero attacks are manual
- Combat events system for processing damage and deaths
- Combat component with attack damage values
- Attacks cost 1 AP; Hero must click a monster to attack (must be adjacent)

### Health System (`health.rs`)
- Health component with current/max HP and healing mechanics
- Low health triggers behavioral changes in monsters
- Death handling and respawn mechanics

### Input System (`input.rs`)
- Centralized input handling for game controls
- Keyboard and mouse event processing
- Input state management and event dispatching

### Pathfinding System (`pathfinding.rs`)
- A* pathfinding algorithm for hexagonal grids
- Uses hexx library for hexagonal coordinate calculations
- Considers terrain movement costs and passability
- Returns optimal paths respecting tile properties

### UI System (`ui/`)
- **Terminal Interface**: Scrollable terminal with game logs and status messages
- **Turn Display**: Shows current turn number and phase
- **Hero Status**: Displays hero action points and stats
- **Advanced Scrollbar**: Custom scrollbar with proper drag, wheel, and click support
- **System Ordering**: Mouse wheel events prioritize terminal over camera when over terminal
- **Dynamic Layout**: Adapts to font size changes and window resizing
- **Event Logging**: Comprehensive game event logging with timestamps

### Camera System (`helpers/camera.rs`)
- WASD movement controls
- Z/X keyboard zoom + mouse wheel zoom
- Movement speed scales with zoom level
- Orthographic projection with scale bounds (0.1-5.0)
- **System Ordering**: Runs after terminal scroll to prevent interference

### Picking System (`helpers/picking.rs`)
- Custom tilemap picking backend for Bevy
- Converts screen coordinates to tile coordinates
- Handles click events on hexagonal tiles
- Supports both left and right click actions

### Main Game Loop (`main.rs`)
- Creates 20x20 hexagonal tilemap
- Initializes different terrain types based on position
- Spawns hero at center position (10, 10)
- Sets up click handlers for hero selection and movement
- Integrates all game systems via plugins

## Build Configuration

- Uses nightly Rust toolchain (specified in rust-toolchain.toml)
- Dynamic linking enabled for faster development builds
- Edition 2024 support

## Development Notes

- Game uses hexagonal coordinate system (HexCoordSystem::Row)
- Tiles are 16x16 pixels with center anchor
- Heroes represented as blue squares, monsters as red squares
- **Turn-based timing**: Spawning and turn logic are turn-based, but movement has smooth animation
- Movement animation is smooth but logical position updates happen discretely
- Monster AI only processes during EnemyTurn phase
- Camera controls remain real-time for responsive user experience
- Movement costs vary by terrain type (grass=1, mountain=3, water=impassable)
- Asset loading uses "colored_packed.png" tileset
- Extensive tile type system ready for future expansion

## Gameplay Features

- **Action Points**: Movement and attacks consume AP; Hero default AP = 6, Monsters = 4; AP refreshes at the start of each side's turn
- **Monster AI**: Monsters spawn every 3 turns and use sight-based AI, only acting during EnemyTurn phase
- **Combat**: Mixed system — monsters attack during EnemyTurn via AI; hero attacks are manual, cost 1 AP, and require adjacency
- **Health & Death**: Heroes and monsters have health/attack values with kill tracking and healing mechanics
- **Pathfinding**: Click-to-move with automatic path calculation
- **Terrain Effects**: Different terrain types affect movement cost
- **Hero Selection**: Click on the hero to select/deselect; selection is reflected in the HUD
- **Advanced Terminal UI**:
  - Scrollable terminal with game logs and event history
  - Custom scrollbar with drag, click, and mouse wheel support
  - No overscroll - proper bounds enforcement
  - Dynamic content sizing and layout adaptation
  - Mouse wheel isolation - terminal scroll doesn't affect map

## Controls (Runtime)

- **WASD**: Move camera
- **Z**: Zoom out (keyboard)
- **X**: Zoom in (keyboard)
- **Mouse wheel**: Zoom in/out (only when not over terminal)
- **Left click on hero**: Select/deselect hero
- **Left click on tile**: Move selected hero to that tile (if possible)
- **Right click on tile**: Cycle through terrain types
- **Space**: End current turn

### Terminal UI Controls:
- **Mouse wheel**: Scroll terminal content (when mouse over terminal)
- **Click scrollbar track**: Jump to position
- **Drag scrollbar thumb**: Smooth scrolling
- **Automatic scrolling**: Terminal auto-scrolls to show new messages

## Technical Implementation Notes

### Scrollbar System Architecture
- **ScrollbarMetrics**: Centralized calculation system for all scroll operations
- **Dynamic font detection**: Uses actual font size from text components
- **Robust content sizing**: Prefers computed layout size with intelligent fallback estimation
- **System ordering**: Terminal scroll system runs before camera system to prevent interference
- **Overscroll prevention**: Strict bounds checking prevents scrolling past content limits
- **Real-time updates**: Scrollbar position and size update during drag operations without flickering

### Event Handling Priority
- Mouse wheel events are processed by terminal first when mouse is over terminal
- Camera zoom only processes mouse wheel events when terminal doesn't handle them
- Clean separation prevents both systems from processing the same scroll event

### Performance Optimizations
- Clippy lints configured to allow complex function signatures and type complexity
- Efficient query systems with proper filtering to avoid unnecessary computations
- Dynamic content height calculation only when needed