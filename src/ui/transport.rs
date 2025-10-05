use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::ui::mode::GameMode;
use crate::economy::{PlaceImprovement, ImprovementKind};
use crate::ui::logging::TerminalLogEvent;

#[derive(Component)]
pub struct TransportScreen;

#[derive(Resource, Default)]
pub struct TransportToolState {
    pub first: Option<TilePos>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct TransportSelectTile {
    pub pos: TilePos,
}

pub struct TransportUIPlugin;

impl Plugin for TransportUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransportToolState>()
            .add_message::<TransportSelectTile>()
            .add_systems(OnEnter(GameMode::Transport), ensure_transport_screen_visible)
            .add_systems(OnExit(GameMode::Transport), hide_transport_screen)
            .add_systems(Update, handle_transport_selection.run_if(in_state(GameMode::Transport)));
    }
}

pub fn handle_transport_selection(
    mut ev: MessageReader<TransportSelectTile>,
    mut tool: ResMut<TransportToolState>,
    mut place_writer: MessageWriter<PlaceImprovement>,
    mut log: MessageWriter<TerminalLogEvent>,
) {
    for e in ev.read() {
        if let Some(a) = tool.first.take() {
            // second click
            let b = e.pos;
            place_writer.write(PlaceImprovement { a, b, kind: ImprovementKind::Road });
        } else {
            tool.first = Some(e.pos);
            log.write(TerminalLogEvent { message: format!("Selected tile ({}, {}) for road start", e.pos.x, e.pos.y) });
        }
    }
}

pub fn ensure_transport_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<TransportScreen>>,
) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

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
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
            TransportScreen,
            Visibility::Visible,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Transport Mode: Allocate transport capacity"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.9, 0.95, 1.0)),
            ));

            // TODO: Add capacity allocation sliders here

            // Back to Map
            parent.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                crate::ui::mode::MapModeButton,
            )).with_children(|b| {
                b.spawn((
                    Text::new("Back to Map"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                ));
            });
        });
}

pub fn hide_transport_screen(mut roots: Query<&mut Visibility, With<TransportScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
