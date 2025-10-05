use bevy::app::AppExit;
use bevy::prelude::*;

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

/// Marker for "New Game" button
#[derive(Component)]
pub struct NewGameButton;

/// Marker for "Quit" button
#[derive(Component)]
pub struct QuitButton;

pub struct MenuUIPlugin;

impl Plugin for MenuUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), ensure_main_menu_visible)
            .add_systems(OnExit(AppState::MainMenu), hide_main_menu)
            .add_systems(Update, handle_menu_buttons.run_if(in_state(AppState::MainMenu)));
    }
}

fn ensure_main_menu_visible(mut commands: Commands, mut existing: Query<&mut Visibility, With<MainMenuRoot>>) {
    if let Ok(mut vis) = existing.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    // Fullscreen menu background panel
    commands
        .spawn((
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
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Rust Imperialism"),
                TextFont { font_size: 36.0, ..default() },
                TextColor(Color::srgb(1.0, 0.95, 0.85)),
                Node { margin: UiRect::bottom(Val::Px(16.0)), ..default() },
            ));

            // New Game button
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                    NewGameButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("New Game"),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });

            // Quit button
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                    QuitButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Quit"),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
        });
}

fn hide_main_menu(mut roots: Query<&mut Visibility, With<MainMenuRoot>>) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Hidden;
    }
}

fn handle_menu_buttons(
    mut interactions: Query<(&Interaction, Option<&NewGameButton>, Option<&QuitButton>), Changed<Interaction>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    for (interaction, is_new_game, is_quit) in interactions.iter_mut() {
        if *interaction == Interaction::Pressed {
            if is_new_game.is_some() {
                next_app_state.set(AppState::InGame);
            } else if is_quit.is_some() {
                exit_writer.write(AppExit::Success);
            }
        }
    }
}
