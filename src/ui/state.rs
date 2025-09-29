use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::health::Health;
use crate::hero::Hero;
use crate::monster::Monster;
use crate::movement::ActionPoints;
use crate::turn_system::{TurnPhase, TurnSystem};

/// Centralized UI state that consolidates all game state needed by UI systems
/// This reduces the number of queries each UI system needs to perform
#[derive(Resource, Default, Debug)]
pub struct UIState {
    pub hero: Option<HeroState>,
    pub turn: TurnState,
    pub monster_count: usize,
}

#[derive(Debug, Clone)]
pub struct HeroState {
    pub is_selected: bool,
    pub health: HealthState,
    pub action_points: ActionPointsState,
    pub kills: u32,
    pub position: TilePos,
}

#[derive(Debug, Clone)]
pub struct HealthState {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct ActionPointsState {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct TurnState {
    pub current_turn: u32,
    pub phase: TurnPhase,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            current_turn: 1,
            phase: TurnPhase::PlayerTurn,
        }
    }
}

impl From<&Health> for HealthState {
    fn from(health: &Health) -> Self {
        Self {
            current: health.current,
            max: health.max,
        }
    }
}

impl From<&ActionPoints> for ActionPointsState {
    fn from(action_points: &ActionPoints) -> Self {
        Self {
            current: action_points.current,
            max: action_points.max,
        }
    }
}

impl From<&TurnSystem> for TurnState {
    fn from(turn_system: &TurnSystem) -> Self {
        Self {
            current_turn: turn_system.current_turn,
            phase: turn_system.phase,
        }
    }
}

impl UIState {
    /// Update all UI state from game world queries
    pub fn update(
        &mut self,
        hero_query: Option<(&Hero, &Health, &ActionPoints, &TilePos)>,
        turn_system: &TurnSystem,
        monster_count: usize,
    ) {
        // Update hero state
        self.hero = hero_query.map(|(hero, health, action_points, position)| HeroState {
            is_selected: hero.is_selected,
            health: health.into(),
            action_points: action_points.into(),
            kills: hero.kills,
            position: *position,
        });

        // Update turn state
        self.turn = turn_system.into();

        // Update monster count
        self.monster_count = monster_count;
    }

    /// Check if any UI-relevant state has changed
    pub fn needs_update(
        &self,
        hero_query: Option<(&Hero, &Health, &ActionPoints, &TilePos)>,
        turn_system: &TurnSystem,
        monster_count: usize,
    ) -> bool {
        // Check if turn state changed
        if self.turn.current_turn != turn_system.current_turn
            || self.turn.phase != turn_system.phase
        {
            return true;
        }

        // Check if monster count changed
        if self.monster_count != monster_count {
            return true;
        }

        // Check if hero state changed
        match (hero_query, &self.hero) {
            (Some((hero, health, action_points, position)), Some(cached_hero)) => {
                hero.is_selected != cached_hero.is_selected
                    || health.current != cached_hero.health.current
                    || health.max != cached_hero.health.max
                    || action_points.current != cached_hero.action_points.current
                    || action_points.max != cached_hero.action_points.max
                    || hero.kills != cached_hero.kills
                    || *position != cached_hero.position
            }
            (Some(_), None) => true, // Hero appeared
            (None, Some(_)) => true, // Hero disappeared
            (None, None) => false,   // No hero, no change
        }
    }

    /// Get formatted turn display text
    pub fn turn_display_text(&self) -> String {
        let phase_text = match self.turn.phase {
            TurnPhase::PlayerTurn => "Player Turn",
            TurnPhase::Processing => "Processing",
            TurnPhase::EnemyTurn => "Enemy Turn",
        };
        format!("Turn: {} - {}", self.turn.current_turn, phase_text)
    }

    /// Get formatted hero status display text
    pub fn hero_status_text(&self) -> String {
        if let Some(hero) = &self.hero {
            let selection_text = if hero.is_selected { " [SELECTED]" } else { "" };
            format!(
                "Hero: HP {}/{}, AP {}/{}, Kills: {}{}",
                hero.health.current,
                hero.health.max,
                hero.action_points.current,
                hero.action_points.max,
                hero.kills,
                selection_text
            )
        } else {
            "No Hero".to_string()
        }
    }

    /// Get monster count text
    pub fn monster_count_text(&self) -> String {
        format!("Monsters: {}", self.monster_count)
    }
}

/// System to collect game state and update the centralized UIState resource
pub fn collect_ui_state(
    mut ui_state: ResMut<UIState>,
    hero_query: Query<(&Hero, &Health, &ActionPoints, &TilePos), With<Hero>>,
    monster_query: Query<&Monster>,
    turn_system: Res<TurnSystem>,
) {
    let hero_data = hero_query.iter().next();
    let monster_count = monster_query.iter().count();

    // Only update if something has changed to avoid unnecessary UI updates
    if ui_state.needs_update(hero_data, &turn_system, monster_count) {
        ui_state.update(hero_data, &turn_system, monster_count);
    }
}

/// Event to notify UI systems that state has been updated
#[derive(Event)]
pub struct UIStateUpdated;

/// System to send UI state update events when state changes
pub fn notify_ui_state_changes(
    ui_state: Res<UIState>,
    mut state_events: EventWriter<UIStateUpdated>,
) {
    if ui_state.is_changed() && !ui_state.is_added() {
        state_events.write(UIStateUpdated);
    }
}

#[cfg(test)]
mod tests;
