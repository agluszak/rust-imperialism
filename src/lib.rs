//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

pub mod combat;
pub mod constants;
pub mod health;
pub mod helpers;
pub mod hero;
pub mod input;
pub mod monster;
pub mod movement;
pub mod pathfinding;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;
pub mod turn_system;
pub mod ui;

#[cfg(test)]
pub mod test_utils;
