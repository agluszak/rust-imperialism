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
├── ui.rs             # Game UI for turn/hero status display
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
- Hero movement points refresh at start of each turn

### Hero System (`hero.rs`)
- `Hero` component with name, movement points, selection state, and kill tracking
- `HeroMovement` component for pathfinding with smooth animation
- Heroes spawn as blue squares on the tilemap
- Selection indicated by yellow color
- Movement points consumed based on terrain cost
- Manual attack system that costs 1 MP - click on monster to attack

### Monster System (`monster.rs`)
- `Monster` component with name, sight range, behavior types, and spawn turn tracking
- AI behaviors: Aggressive (attacks on sight), Defensive (attacks when close), Fleeing (retreats when low HP)
- Turn-based AI decisions made only during EnemyTurn phase
- Smooth animation for movement with logical position updates
- Spawns every 3 turns with different monster types (Goblin, Orc, Skeleton)

### Combat System (`combat.rs`)
- Manual combat system - no automatic attacks
- Combat events system for processing damage and deaths
- Combat component with attack damage values
- Hero attacks cost 1 MP and must be initiated by clicking on monsters

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

### UI System (`ui.rs`)
- Displays current turn number and phase
- Shows hero status including movement points
- Updates in real-time as game state changes
- Simple text-based interface in top-left corner

### Camera System (`helpers/camera.rs`)
- WASD movement controls
- Z/X keyboard zoom + mouse wheel zoom
- Movement speed scales with zoom level
- Orthographic projection with scale bounds (0.1-5.0)

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

- **Turn-based Movement**: Heroes have limited movement points per turn with smooth animated movement
- **Monster AI**: Monsters spawn every 3 turns and use sight-based AI, only acting during EnemyTurn phase
- **Manual Combat System**: Heroes must manually attack monsters by clicking on them (costs 1 MP)
- **Health & Death**: Heroes and monsters have health/attack values with kill tracking and healing mechanics
- **Pathfinding**: Click-to-move with automatic path calculation
- **Terrain Effects**: Different terrain types affect movement cost
- **Hero Selection**: Click on hero to select/deselect, indicated by color change
- **Turn-based UI**: Turn counter and hero status display

## Controls (Runtime)

- **WASD**: Move camera
- **Z**: Zoom out (keyboard)
- **X**: Zoom in (keyboard)
- **Mouse wheel**: Zoom in/out
- **Left click on hero**: Select/deselect hero
- **Left click on tile**: Move selected hero to that tile (if possible)
- **Right click on tile**: Cycle through terrain types
- **Space**: End current turn