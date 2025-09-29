use crate::constants::*;
use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[derive(Component, Debug, Clone)]
pub struct Combat {
    pub attack_damage: u32,
}

impl Health {
    pub fn new(max_hp: u32) -> Self {
        Self {
            current: max_hp,
            max: max_hp,
        }
    }

    pub fn take_damage(&mut self, damage: u32) {
        self.current = self.current.saturating_sub(damage);
    }

    pub fn heal_to_full(&mut self) {
        self.current = self.max;
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn is_low_health(&self) -> bool {
        self.current <= (self.max * LOW_HEALTH_THRESHOLD_PERCENT) / 100
    }

    pub fn heal(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn percentage(&self) -> u32 {
        if self.max == 0 {
            0
        } else {
            (self.current * 100) / self.max
        }
    }
}

impl Combat {
    pub fn new(attack_damage: u32) -> Self {
        Self { attack_damage }
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Default for Combat {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests;
