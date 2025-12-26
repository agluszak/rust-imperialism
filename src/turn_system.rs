use bevy::prelude::*;

use crate::diplomacy::DiplomaticOffers;
use crate::economy::{Calendar, PlayerNation, Season};
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

// ============================================================================
// Core Turn State Types
// ============================================================================

/// The current turn number. Increments after each full turn cycle.
#[derive(Resource, Debug, Clone, Reflect, Default)]
#[reflect(Resource)]
pub struct TurnCounter {
    pub current: u32,
}

impl TurnCounter {
    pub fn new(turn: u32) -> Self {
        Self { current: turn }
    }

    pub fn increment(&mut self) {
        self.current += 1;
    }
}

/// Turn phase as a Bevy State. Transitions fire OnEnter/OnExit exactly once.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum TurnPhase {
    /// Player can issue orders, move units, etc.
    #[default]
    PlayerTurn,
    /// Orders are executed, production happens, allocations finalize.
    Processing,
    /// AI nations take their turns.
    EnemyTurn,
}

// ============================================================================
// System Sets for Turn Phase Ordering
// ============================================================================

/// Systems that run when entering PlayerTurn (start of a new turn).
/// Order: Collection → Feeding → Market → Reset
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PlayerTurnSet {
    /// Collect resources from connected tiles
    Collection,
    /// Feed workers, apply recurring effects
    Maintenance,
    /// Resolve market orders from previous turn
    Market,
    /// Reset allocations for new turn
    Reset,
    /// Update UI state
    Ui,
}

/// Systems that run during Processing phase.
/// Order: Finalize → Production → Conversion
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ProcessingSet {
    /// Commit reservations
    Finalize,
    /// Run production, execute orders
    Production,
    /// Convert goods to capacity
    Conversion,
}

/// Systems that run during EnemyTurn.
/// Order: Setup → Analysis → Decisions → Actions
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum EnemyTurnSet {
    /// Reset AI state, rebuild context
    Setup,
    /// AI analyzes situation
    Analysis,
    /// AI makes decisions (scorers)
    Decisions,
    /// AI executes actions
    Actions,
    /// AI emits orders
    Orders,
}

// ============================================================================
// Commands for Turn Control
// ============================================================================

/// Command to end the player's turn and begin processing.
#[derive(Message, Debug, Clone)]
pub struct EndPlayerTurn;

// ============================================================================
// Plugin
// ============================================================================

pub struct TurnSystemPlugin;

impl Plugin for TurnSystemPlugin {
    fn build(&self, app: &mut App) {
        // Register state and resources
        app.init_state::<TurnPhase>()
            .insert_resource(TurnCounter::new(1))
            .add_message::<EndPlayerTurn>();

        // Configure system set ordering for PlayerTurn
        app.configure_sets(
            OnEnter(TurnPhase::PlayerTurn),
            (
                PlayerTurnSet::Collection,
                PlayerTurnSet::Maintenance,
                PlayerTurnSet::Market,
                PlayerTurnSet::Reset,
                PlayerTurnSet::Ui,
            )
                .chain(),
        );

        // Configure system set ordering for Processing
        app.configure_sets(
            OnEnter(TurnPhase::Processing),
            (
                ProcessingSet::Finalize,
                ProcessingSet::Production,
                ProcessingSet::Conversion,
            )
                .chain(),
        );

        // Configure system set ordering for EnemyTurn
        app.configure_sets(
            OnEnter(TurnPhase::EnemyTurn),
            (
                EnemyTurnSet::Setup,
                EnemyTurnSet::Analysis,
                EnemyTurnSet::Decisions,
                EnemyTurnSet::Actions,
                EnemyTurnSet::Orders,
            )
                .chain(),
        );

        // Logging systems for phase transitions
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            log_turn_start.before(PlayerTurnSet::Collection),
        );

        app.add_systems(
            OnEnter(TurnPhase::Processing),
            log_processing_start.before(ProcessingSet::Finalize),
        );

        app.add_systems(
            OnEnter(TurnPhase::EnemyTurn),
            log_enemy_turn_start.before(EnemyTurnSet::Setup),
        );

        // Auto-transition: Processing → EnemyTurn (after all Processing systems)
        app.add_systems(
            OnEnter(TurnPhase::Processing),
            transition_to_enemy_turn.after(ProcessingSet::Conversion),
        );

        // Auto-transition: EnemyTurn → PlayerTurn (after all EnemyTurn systems)
        app.add_systems(
            OnEnter(TurnPhase::EnemyTurn),
            transition_to_next_turn.after(EnemyTurnSet::Orders),
        );

        // Input handling (runs every frame during gameplay)
        app.add_systems(
            Update,
            handle_end_turn_input
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(TurnPhase::PlayerTurn)),
        );

        // Transition command handler for player ending turn
        app.add_systems(
            Update,
            handle_end_player_turn.run_if(in_state(AppState::InGame)),
        );

        // Calendar advancement (on new turn)
        app.add_systems(
            OnEnter(TurnPhase::PlayerTurn),
            advance_calendar.in_set(PlayerTurnSet::Maintenance),
        );
    }
}

// ============================================================================
// Logging Systems
// ============================================================================

fn log_turn_start(turn: Res<TurnCounter>) {
    info!("=== Turn {} - PlayerTurn ===", turn.current);
}

fn log_processing_start(turn: Res<TurnCounter>) {
    info!("=== Turn {} - Processing ===", turn.current);
}

fn log_enemy_turn_start(turn: Res<TurnCounter>) {
    info!("=== Turn {} - EnemyTurn ===", turn.current);
}

// ============================================================================
// Input Handling
// ============================================================================

fn handle_end_turn_input(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    offers: Option<Res<DiplomaticOffers>>,
    player: Option<Res<PlayerNation>>,
    game_mode: Option<Res<State<GameMode>>>,
    mut end_turn_events: MessageWriter<EndPlayerTurn>,
) {
    let Some(keys) = keys else {
        return;
    };

    // Only allow ending turn from Map screen
    if let Some(mode) = &game_mode
        && *mode.get() != GameMode::Map
    {
        return;
    }

    if keys.just_pressed(KeyCode::Space) {
        // Check for pending diplomatic offers
        if let (Some(offers), Some(player)) = (offers, player)
            && offers.has_pending_for(player.instance())
        {
            info!("Resolve pending diplomatic offers before ending the turn.");
            return;
        }
        end_turn_events.write(EndPlayerTurn);
    }
}

// ============================================================================
// Transition Handlers
// ============================================================================

fn handle_end_player_turn(
    mut messages: MessageReader<EndPlayerTurn>,
    mut next_state: ResMut<NextState<TurnPhase>>,
) {
    for _ in messages.read() {
        info!("Player turn ended, beginning processing...");
        next_state.set(TurnPhase::Processing);
    }
}

// ============================================================================
// Auto-Transition Systems
// ============================================================================

/// Automatically transitions from Processing to EnemyTurn.
/// Runs at the end of OnEnter(Processing) after all processing systems complete.
fn transition_to_enemy_turn(mut next_state: ResMut<NextState<TurnPhase>>) {
    info!("Processing complete, beginning enemy turn...");
    next_state.set(TurnPhase::EnemyTurn);
}

/// Automatically transitions from EnemyTurn to next PlayerTurn.
/// Runs at the end of OnEnter(EnemyTurn) after all AI systems complete.
fn transition_to_next_turn(
    mut next_state: ResMut<NextState<TurnPhase>>,
    mut turn: ResMut<TurnCounter>,
) {
    turn.increment();
    info!("Enemy turn complete, beginning turn {}...", turn.current);
    next_state.set(TurnPhase::PlayerTurn);
}

// ============================================================================
// Calendar
// ============================================================================

fn advance_calendar(mut calendar: Option<ResMut<Calendar>>, turn: Res<TurnCounter>) {
    // Only advance calendar after turn 1 (first turn doesn't advance)
    if turn.current <= 1 {
        return;
    }

    if let Some(cal) = calendar.as_mut() {
        cal.season = match cal.season {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => {
                cal.year = cal.year.saturating_add(1);
                Season::Spring
            }
        };
    }
}

#[cfg(test)]
mod tests;
