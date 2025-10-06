use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Technology {
    // Rail construction technologies
    MountainEngineering, // Allows building rails in mountains
    SwampDrainage,       // Allows building rails in swamps
    HillGrading,         // Allows building rails in hills
}

/// Set of technologies owned by a nation
#[derive(Component, Debug, Default, Clone)]
pub struct Technologies(pub HashSet<Technology>);

impl Technologies {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn has(&self, tech: Technology) -> bool {
        self.0.contains(&tech)
    }

    pub fn unlock(&mut self, tech: Technology) {
        self.0.insert(tech);
    }
}
