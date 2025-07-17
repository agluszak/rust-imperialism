# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based hexagonal tile-based game called "rust-imperialism" built with the Bevy game engine. The project appears to be an early-stage imperialism/strategy game with hexagonal tiles that can be clicked to cycle through different terrain types.

## Key Dependencies

- **Bevy 0.16**: Main game engine with dynamic linking, dev tools, and mesh picking backend
- **hexx 0.21**: Hexagonal grid algorithms and utilities with Bevy integration
- **bevy_ecs_tilemap**: Custom tilemap rendering system from GitHub (main branch)
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
- `Hero` component with name, movement points, and selection state
- `HeroMovement` component for pathfinding and animation
- Heroes spawn as blue squares on the tilemap
- Selection indicated by yellow color
- Movement points consumed based on terrain cost

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

- Uses nightly Rust toolchain
- Optimized for fast compilation with `mold` linker on Linux
- Dynamic linking enabled for faster development builds
- Custom cargo config for Windows/Linux builds

## Development Notes

- Game uses hexagonal coordinate system (HexCoordSystem::Row)
- Tiles are 16x16 pixels with center anchor
- Hero represented as blue square sprite with collision detection
- Turn-based movement with pathfinding validation
- Movement costs vary by terrain type (grass=1, mountain=3, water=impassable)
- Asset loading uses "colored_packed.png" tileset
- Extensive tile type system ready for future expansion

## Gameplay Features

- **Turn-based Movement**: Heroes have limited movement points per turn
- **Pathfinding**: Click-to-move with automatic path calculation
- **Terrain Effects**: Different terrain types affect movement cost
- **Hero Selection**: Click on hero to select/deselect, indicated by color change
- **Real-time UI**: Turn counter and hero status display

## Controls (Runtime)

- **WASD**: Move camera
- **Z**: Zoom out (keyboard)
- **X**: Zoom in (keyboard)
- **Mouse wheel**: Zoom in/out
- **Left click on hero**: Select/deselect hero
- **Left click on tile**: Move selected hero to that tile (if possible)
- **Right click on tile**: Cycle through terrain types
- **Space**: End current turn