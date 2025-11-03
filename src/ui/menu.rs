use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button, observe};

use crate::ui::button_style::*;
use crate::ui::generic_systems::hide_screen;

/// Root application state controlling whether we're in the Main Menu or the actual game
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default, Reflect)]
pub enum AppState {
    /// Start menu
    #[default]
    MainMenu,
    /// Gameplay (Map/City/etc.)
    InGame,
}

/// Marker for the root of the Main Menu UI
#[derive(Component)]
pub struct MainMenuRoot;

/// Creates an observer that quits the application when button is activated
pub fn quit_game() -> impl Bundle {
    observe(
        |_activate: On<Activate>, mut exit: MessageWriter<AppExit>| {
            info!("Quit button activated - exiting application");
            exit.write(AppExit::Success);
        },
    )
}

pub struct MenuUIPlugin;

impl Plugin for MenuUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), ensure_main_menu_visible)
            .add_systems(OnExit(AppState::MainMenu), hide_screen::<MainMenuRoot>);
    }
}

fn ensure_main_menu_visible(
    mut commands: Commands,
    mut existing: Query<&mut Visibility, With<MainMenuRoot>>,
) {
    if let Ok(mut vis) = existing.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    // Fullscreen menu background panel
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            padding: UiRect::all(Val::Px(16.0)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.96)),
        MainMenuRoot,
        Visibility::Visible,
        children![
            (
                Text::new("Rust Imperialism"),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.85)),
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ),
            (
                Button,
                OldButton,
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_ACCENT),
                AccentButton,
                observe(
                    |_activate: On<Activate>, mut next_state: ResMut<NextState<AppState>>| {
                        info!("New Game button activated - switching to InGame state");
                        next_state.set(AppState::InGame);
                    }
                ),
                children![(
                    Text::new("New Game"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ),
            (
                Button,
                OldButton,
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                quit_game(),
                children![(
                    Text::new("Quit"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ),
        ],
    ));
}
