use bevy::prelude::*;

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
        app.init_resource::<TurnSystem>()
            .add_systems(Update, (
                handle_turn_input,
                process_turn_phases,
                update_turn_display,
            ));
    }
}

fn handle_turn_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut turn_system: ResMut<TurnSystem>,
) {
    if keys.just_pressed(KeyCode::Space) && turn_system.is_player_turn() {
        turn_system.end_player_turn();
        println!("Player turn ended. Turn: {}", turn_system.current_turn);
    }
}

fn process_turn_phases(
    mut turn_system: ResMut<TurnSystem>,
) {
    // Auto-advance from Processing to EnemyTurn after a short delay
    if turn_system.phase == TurnPhase::Processing {
        // For now, immediately advance to PlayerTurn (skipping enemy turn)
        // In a real game, you'd process enemy AI here
        turn_system.advance_turn(); // Processing -> EnemyTurn
        turn_system.advance_turn(); // EnemyTurn -> PlayerTurn with new turn number
        println!("Starting turn {}", turn_system.current_turn);
    }
}

fn update_turn_display(
    turn_system: Res<TurnSystem>,
) {
    if turn_system.is_changed() {
        println!("=== Turn {} - {:?} ===", turn_system.current_turn, turn_system.phase);
    }
}