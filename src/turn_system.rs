use bevy::prelude::*;

use crate::ui::logging::TerminalLogEvent;

#[derive(Resource, Debug, Clone)]
pub struct TurnSystem {
    pub current_turn: u32,
    pub phase: TurnPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnPhase {
    PlayerTurn,
    Processing,
    EnemyTurn,
}

impl Default for TurnSystem {
    fn default() -> Self {
        Self {
            current_turn: 1,
            phase: TurnPhase::PlayerTurn,
        }
    }
}

impl TurnSystem {
    pub fn advance_turn(&mut self) {
        match self.phase {
            TurnPhase::PlayerTurn => self.phase = TurnPhase::Processing,
            TurnPhase::Processing => self.phase = TurnPhase::EnemyTurn,
            TurnPhase::EnemyTurn => {
                self.current_turn += 1;
                self.phase = TurnPhase::PlayerTurn;
            }
        }
    }

    pub fn end_player_turn(&mut self) {
        if self.phase == TurnPhase::PlayerTurn {
            self.phase = TurnPhase::Processing;
        }
    }

    pub fn is_player_turn(&self) -> bool {
        self.phase == TurnPhase::PlayerTurn
    }
}

pub struct TurnSystemPlugin;

impl Plugin for TurnSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TurnSystem>().add_systems(
            Update,
            (handle_turn_input, process_turn_phases, update_turn_display),
        );
    }
}

fn handle_turn_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut turn_system: ResMut<TurnSystem>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    if keys.just_pressed(KeyCode::Space) && turn_system.is_player_turn() {
        turn_system.end_player_turn();
        log_writer.write(TerminalLogEvent {
            message: format!("Player turn ended. Turn: {}", turn_system.current_turn),
        });
    }
}

fn process_turn_phases(
    mut turn_system: ResMut<TurnSystem>,
    mut turn_timer: Local<Timer>,
    time: Res<Time>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    // Handle turn phase transitions with timing
    match turn_system.phase {
        TurnPhase::Processing => {
            // Short delay before enemy turn
            if turn_timer.duration().is_zero() {
                *turn_timer = Timer::from_seconds(0.5, TimerMode::Once);
            }

            turn_timer.tick(time.delta());

            if turn_timer.just_finished() {
                turn_system.advance_turn(); // Processing -> EnemyTurn
                log_writer.write(TerminalLogEvent {
                    message: "=== Enemy Turn ===".to_string(),
                });
                turn_timer.reset();
            }
        }
        TurnPhase::EnemyTurn => {
            // Give monsters time to act, then advance
            if turn_timer.duration().is_zero() {
                *turn_timer = Timer::from_seconds(2.0, TimerMode::Once);
            }

            turn_timer.tick(time.delta());

            if turn_timer.just_finished() {
                turn_system.advance_turn(); // EnemyTurn -> PlayerTurn with new turn number
                log_writer.write(TerminalLogEvent {
                    message: format!("Starting turn {}", turn_system.current_turn),
                });
                turn_timer.reset();
            }
        }
        _ => {}
    }
}

fn update_turn_display(
    turn_system: Res<TurnSystem>,
    mut log_writer: EventWriter<TerminalLogEvent>,
) {
    if turn_system.is_changed() {
        log_writer.write(TerminalLogEvent {
            message: format!(
                "=== Turn {} - {:?} ===",
                turn_system.current_turn, turn_system.phase
            ),
        });
    }
}
