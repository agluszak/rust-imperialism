use bevy::prelude::*;

use crate::ui::components::TerminalOutput;

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

#[derive(Event, Clone, Debug)]
pub struct TerminalLogEvent {
    pub message: String,
}

pub fn setup_terminal_log(mut writer: EventWriter<TerminalLogEvent>) {
    writer.write(TerminalLogEvent {
        message: "=== Game Controls ===".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "WASD: Move camera".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Z: Zoom out (keyboard)".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "X: Zoom in (keyboard)".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Mouse wheel: Zoom in/out".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Left click: Select hero or move hero".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Right click: Cycle terrain types".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Space: End turn".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "Use scrollbar to scroll terminal".to_string(),
    });
    writer.write(TerminalLogEvent {
        message: "=====================".to_string(),
    });

    // Add more content to test scrolling
    for i in 1..=30 {
        writer.write(TerminalLogEvent {
            message: format!(
                "Test message {} - this is a longer message to demonstrate scrolling behavior",
                i
            ),
        });
    }
}

pub fn consume_log_events(
    mut reader: EventReader<TerminalLogEvent>,
    mut terminal_log: ResMut<TerminalLog>,
) {
    for ev in reader.read() {
        terminal_log.add_message(ev.message.clone());
    }
}

pub fn update_terminal_output(
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

// Helper function to emit log events
pub fn emit_log(mut writer: EventWriter<TerminalLogEvent>, message: impl Into<String>) {
    writer.write(TerminalLogEvent {
        message: message.into(),
    });
}
