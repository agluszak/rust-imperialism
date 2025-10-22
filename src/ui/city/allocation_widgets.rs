use bevy::prelude::*;

use crate::economy::{Good, WorkerSkill};

// Note: ChildBuilder is part of Bevy's UI hierarchy building
// If needed, we can define it explicitly, but it's usually in prelude

// ============================================================================
// Core Types: Unified allocation identification
// ============================================================================

/// Identifies what this allocation UI controls
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AllocationType {
    Recruitment,
    Training(WorkerSkill),
    Production(Entity, Good), // building entity + output good
    MarketBuy(Good),
    MarketSell(Good),
}

/// Generic stepper display (shows current allocated value)
#[derive(Component, Clone, Copy)]
pub struct AllocationStepperDisplay {
    pub allocation_type: AllocationType,
}

/// Generic stepper button
#[derive(Component, Clone, Copy)]
pub struct AllocationStepperButton {
    pub allocation_type: AllocationType,
    pub delta: i32,
}

/// Generic allocation bar (for resource requirements)
#[derive(Component, Clone)]
pub struct AllocationBar {
    pub allocation_type: AllocationType,
    pub good: Good,
    pub label: String,
}

/// Generic summary text ("Will do X next turn")
#[derive(Component, Clone, Copy)]
pub struct AllocationSummary {
    pub allocation_type: AllocationType,
}

// ============================================================================
// Widget Configuration
// ============================================================================

pub struct StepperConfig {
    pub label: &'static str,
    pub allocation_type: AllocationType,
    pub small_step: i32,
    pub large_step: i32,
}

pub struct AllocationBarConfig {
    pub good: Good,
    pub good_name: &'static str,
    pub allocation_type: AllocationType,
}

pub struct AllocationSummaryConfig {
    pub allocation_type: AllocationType,
}

// ============================================================================
// Widget Spawn Functions
// ============================================================================

// Note: These spawn functions are meant to be called INSIDE with_children closures
// The parameter is implicitly typed by Bevy's closure context

/// Macro to spawn allocation stepper controls
/// Usage: spawn_allocation_stepper!(parent, label, allocation_type)
#[macro_export]
macro_rules! spawn_allocation_stepper {
    ($parent:expr, $label:expr, $allocation_type:expr) => {{
        use bevy::ui::widget::Button as OldButton;
        use bevy::ui_widgets::Button;
        use $crate::ui::button_style::*;
        use $crate::ui::city::allocation_ui_unified::adjust_allocation_on_click;
        use $crate::ui::city::allocation_widgets::{
            AllocationStepperButton, AllocationStepperDisplay,
        };

        // Label
        $parent.spawn((
            Text::new($label),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
            Node {
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
        ));

        // Stepper row
        $parent
            .spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            })
            .with_children(|row| {
                // Only -1 button
                row.spawn((
                    Button,
                    OldButton,
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor::all(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                    AllocationStepperButton {
                        allocation_type: $allocation_type,
                        delta: -1,
                    },
                    adjust_allocation_on_click($allocation_type, -1),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("−"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });

                // Value display
                row.spawn((
                    Text::new("0"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    AllocationStepperDisplay {
                        allocation_type: $allocation_type,
                    },
                    Node {
                        min_width: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));

                // Only +1 button
                row.spawn((
                    Button,
                    OldButton,
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor::all(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                    AllocationStepperButton {
                        allocation_type: $allocation_type,
                        delta: 1,
                    },
                    adjust_allocation_on_click($allocation_type, 1),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("+"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
            });
    }};
}

/// Macro to spawn allocation bars
#[macro_export]
macro_rules! spawn_allocation_bar {
    ($parent:expr, $good:expr, $good_name:expr, $allocation_type:expr) => {{
        use $crate::ui::city::allocation_widgets::AllocationBar;

        $parent
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            })
            .with_children(|container| {
                container.spawn((
                    Text::new(format!("{}: 0 / 0", $good_name)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    AllocationBar {
                        allocation_type: $allocation_type,
                        good: $good,
                        label: $good_name.to_string(),
                    },
                ));

                container.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(20.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8)),
                    BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8)),
                ));
            });
    }};
}

/// Macro to spawn allocation summary
#[macro_export]
macro_rules! spawn_allocation_summary {
    ($parent:expr, $allocation_type:expr) => {{
        use $crate::ui::city::allocation_widgets::AllocationSummary;

        $parent.spawn((
            Text::new("-> ..."),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.9, 1.0)),
            AllocationSummary {
                allocation_type: $allocation_type,
            },
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ));
    }};
}
