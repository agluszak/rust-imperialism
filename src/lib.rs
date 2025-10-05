//! Rust Imperialism - A hexagonal tile-based strategy game
//!
//! This library exposes the core game components for testing and potential reuse.

pub mod constants;
pub mod helpers;
pub mod input;
pub mod terrain_gen;
pub mod tile_pos;
pub mod tiles;
pub mod turn_system;
pub mod economy;
pub mod ui;
pub mod transport_rendering;
pub mod civilians;

#[cfg(test)]
pub mod test_utils;
