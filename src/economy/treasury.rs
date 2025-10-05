use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Treasury(pub i64);

impl Default for Treasury {
    fn default() -> Self {
        Treasury(50_000)
    }
}
