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

    pub fn with_current(current: u32, max: u32) -> Self {
        Self { current, max }
    }

    pub fn take_damage(&mut self, damage: u32) {
        self.current = self.current.saturating_sub(damage);
    }

    pub fn heal(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn heal_to_full(&mut self) {
        self.current = self.max;
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn is_low_health(&self) -> bool {
        self.current <= self.max / 3
    }

    pub fn health_percentage(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
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
        Self::new(1)
    }
}
