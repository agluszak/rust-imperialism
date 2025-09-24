use crate::health::Health;
use crate::hero::Hero;
use crate::turn_system::TurnSystem;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

#[derive(Component)]
pub struct TurnDisplay;

#[derive(Component)]
pub struct HeroStatusDisplay;

#[derive(Component)]
pub struct TerminalWindow;

#[derive(Component)]
pub struct TerminalOutput;

#[derive(Component)]
pub struct ScrollableTerminal;

#[derive(Component)]
pub struct Scrollbar;

#[derive(Component)]
pub struct ScrollbarThumb;

#[derive(Component)]
pub struct ScrollbarTrack;

#[derive(Component)]
pub struct ScrollbarDragStart {
    pub position: Vec2,
    pub scroll_position: Vec2,
}

#[derive(Resource, Default)]
pub struct TerminalLog {
    pub messages: Vec<String>,
    pub max_messages: usize,
}

impl TerminalLog {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }
}

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerminalLog::new(100))
            .add_systems(Startup, (setup_ui, setup_terminal_log))
            .add_systems(
                Update,
                (
                    update_turn_display,
                    update_hero_status_display,
                    update_terminal_output,
                    handle_scrollbar_drag,
                    update_scrollbar.after(handle_scrollbar_drag),
                ),
            );
    }
}

fn setup_ui(mut commands: Commands) {
    // Create UI root for status display
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // Turn display
            parent.spawn((
                Text::new("Turn: 1 - Player Turn"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TurnDisplay,
            ));

            // Hero status display
            parent.spawn((
                Text::new("Hero: HP 10/10, MP 3/3, Kills: 0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                HeroStatusDisplay,
            ));
        });

    // Create terminal window
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(420.0), // Make room for scrollbar
                height: Val::Px(300.0),
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(5.0)),
                flex_direction: FlexDirection::Row, // Changed to row to accommodate scrollbar
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            TerminalWindow,
        ))
        .with_children(|parent| {
            // Content area container
            parent
                .spawn((Node {
                    width: Val::Percent(95.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },))
                .with_children(|content| {
                    // Terminal header
                    content.spawn((
                        Text::new("Terminal Output"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        Node {
                            margin: UiRect::bottom(Val::Px(5.0)),
                            ..default()
                        },
                    ));

                    // Scrollable content area using native Bevy scrolling
                    content
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition::default(),
                            ScrollableTerminal,
                        ))
                        .with_children(|scrollable| {
                            scrollable.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                                TerminalOutput,
                                Node {
                                    align_self: AlignSelf::Stretch,
                                    min_height: Val::Px(1000.0), // Force content to be taller than container
                                    ..default()
                                },
                            ));
                        });
                });

            // Scrollbar
            parent
                .spawn((
                    Node {
                        width: Val::Percent(5.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                    RelativeCursorPosition::default(),
                    ScrollbarTrack,
                ))
                .with_children(|track| {
                    // Scrollbar thumb
                    track.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(20.0), // Initial thumb size
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                        ScrollbarThumb,
                        RelativeCursorPosition::default(),
                    ));
                });
        });
}

fn update_turn_display(
    turn_system: Res<TurnSystem>,
    mut query: Query<&mut Text, With<TurnDisplay>>,
) {
    if turn_system.is_changed() {
        for mut text in query.iter_mut() {
            let phase_text = match turn_system.phase {
                crate::turn_system::TurnPhase::PlayerTurn => "Player Turn",
                crate::turn_system::TurnPhase::Processing => "Processing",
                crate::turn_system::TurnPhase::EnemyTurn => "Enemy Turn",
            };
            text.0 = format!("Turn: {} - {}", turn_system.current_turn, phase_text);
        }
    }
}

fn update_hero_status_display(
    hero_query: Query<(&Hero, &Health), (With<Hero>, Or<(Changed<Hero>, Changed<Health>)>)>,
    mut text_query: Query<&mut Text, With<HeroStatusDisplay>>,
) {
    for (hero, health) in hero_query.iter() {
        for mut text in text_query.iter_mut() {
            let selection_text = if hero.is_selected { " [SELECTED]" } else { "" };
            text.0 = format!(
                "Hero: HP {}/{}, MP {}/{}, Kills: {}{}",
                health.current,
                health.max,
                hero.movement_points,
                hero.max_movement_points,
                hero.kills,
                selection_text
            );
        }
    }
}

fn update_terminal_output(
    terminal_log: Res<TerminalLog>,
    mut query: Query<&mut Text, With<TerminalOutput>>,
) {
    if terminal_log.is_changed() {
        for mut text in query.iter_mut() {
            let mut output = "=== Terminal Output ===\n".to_string();
            // Reverse the messages so newest appear at top
            for message in terminal_log.messages.iter().rev() {
                output.push_str(message);
                output.push('\n');
            }
            text.0 = output;
        }
    }
}

// Helper function to add messages to terminal log
pub fn log_message(terminal_log: &mut ResMut<TerminalLog>, message: String) {
    terminal_log.add_message(message);
}

// System to log events that need world access
pub fn log_event(mut terminal_log: ResMut<TerminalLog>, message: String) {
    terminal_log.add_message(message);
}

fn setup_terminal_log(mut terminal_log: ResMut<TerminalLog>) {
    terminal_log.add_message("=== Game Controls ===".to_string());
    terminal_log.add_message("WASD: Move camera".to_string());
    terminal_log.add_message("Z: Zoom out (keyboard)".to_string());
    terminal_log.add_message("X: Zoom in (keyboard)".to_string());
    terminal_log.add_message("Mouse wheel: Zoom in/out".to_string());
    terminal_log.add_message("Left click: Select hero or move hero".to_string());
    terminal_log.add_message("Right click: Cycle terrain types".to_string());
    terminal_log.add_message("Space: End turn".to_string());
    terminal_log.add_message("Use scrollbar to scroll terminal".to_string());
    terminal_log.add_message("=====================".to_string());

    // Add more content to test scrolling
    for i in 1..=30 {
        terminal_log.add_message(format!(
            "Test message {} - this is a longer message to demonstrate scrolling behavior",
            i
        ));
    }
}

// Removed unused scroll systems - focusing only on scrollbar functionality

// System to update scrollbar position and size based on scroll state
fn update_scrollbar(
    scrollable_query: Query<(&ScrollPosition, &Node, &ComputedNode), With<ScrollableTerminal>>,
    mut thumb_query: Query<
        &mut Node,
        (
            With<ScrollbarThumb>,
            Without<ScrollableTerminal>,
            Without<ScrollbarDragStart>,
        ),
    >,
    terminal_log: Res<TerminalLog>,
) {
    const LINE_HEIGHT: f32 = 15.0; // Approximate line height based on font size
    const VISIBLE_LINES: f32 = 15.0; // Approximate number of visible lines in terminal

    for (scroll_position, _node, computed) in scrollable_query.iter() {
        let total_lines = terminal_log.messages.len() as f32 + 1.0; // +1 for header

        // Calculate thumb size based on line count
        let thumb_size_ratio = if total_lines > VISIBLE_LINES {
            VISIBLE_LINES / total_lines
        } else {
            1.0 // If content fits, thumb takes full height
        };

        // Calculate scroll position ratio
        let content_size = computed.content_size();
        let visible_size = computed.size();
        let scroll_ratio = if content_size.y > visible_size.y {
            scroll_position.offset_y / (content_size.y - visible_size.y)
        } else {
            0.0
        };

        for mut thumb_node in thumb_query.iter_mut() {
            // Update thumb size based on line count ratio
            thumb_node.height = Val::Percent((thumb_size_ratio * 100.0).max(5.0)); // Min 5% height

            // Update thumb position based on scroll position
            let max_thumb_travel = 100.0 - (thumb_size_ratio * 100.0);
            thumb_node.top = Val::Percent(scroll_ratio * max_thumb_travel);
        }
    }
}

// System to handle scrollbar thumb dragging
fn handle_scrollbar_drag(
    mut commands: Commands,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut scrollable_query: Query<
        (&mut ScrollPosition, &Node, &ComputedNode),
        With<ScrollableTerminal>,
    >,
    thumb_query: Query<(Entity, &RelativeCursorPosition), With<ScrollbarThumb>>,
    drag_query: Query<&ScrollbarDragStart, With<ScrollbarThumb>>,
    track_query: Query<&RelativeCursorPosition, (With<ScrollbarTrack>, Without<ScrollbarThumb>)>,
) {
    // Handle clicking on scrollbar track (jump to position)
    if mouse_button_input.just_pressed(MouseButton::Left) {
        for track_cursor in track_query.iter() {
            if let Some(pos) = track_cursor.normalized
                && let Ok((mut scroll_position, _node, computed)) = scrollable_query.single_mut() {
                    let content_size = computed.content_size();
                    let visible_size = computed.size();

                    if content_size.y > visible_size.y {
                        let max_scroll = content_size.y - visible_size.y;
                        scroll_position.offset_y = pos.y * max_scroll;
                    }
                }
        }
    }

    for (thumb_entity, cursor_position) in thumb_query.iter() {
        // Start dragging
        if mouse_button_input.just_pressed(MouseButton::Left)
            && let Some(pos) = cursor_position.normalized
                && let Ok((scroll_position, _, _)) = scrollable_query.single() {
                    commands.entity(thumb_entity).insert(ScrollbarDragStart {
                        position: pos,
                        scroll_position: Vec2::new(
                            scroll_position.offset_x,
                            scroll_position.offset_y,
                        ),
                    });
                }

        // Handle drag
        if mouse_button_input.pressed(MouseButton::Left)
            && let Ok(drag_start) = drag_query.get(thumb_entity)
                && let Some(current_pos) = cursor_position.normalized {
                    let delta_y = current_pos.y - drag_start.position.y;

                    if let Ok((mut scroll_position, _node, computed)) =
                        scrollable_query.single_mut()
                    {
                        let content_size = computed.content_size();
                        let visible_size = computed.size();

                        if content_size.y > visible_size.y {
                            let max_scroll = content_size.y - visible_size.y;
                            // Scale the delta by a factor to make dragging more responsive
                            let scroll_delta = delta_y * max_scroll * 2.0;

                            scroll_position.offset_y = (drag_start.scroll_position.y
                                + scroll_delta)
                                .max(0.0)
                                .min(max_scroll);
                        }
                    }
                }

        // End dragging
        if mouse_button_input.just_released(MouseButton::Left) {
            commands.entity(thumb_entity).remove::<ScrollbarDragStart>();
        }
    }
}
