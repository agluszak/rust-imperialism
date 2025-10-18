use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue, ValueChange, observe};
use bevy_ecs_tilemap::prelude::TilePos;

use super::button_style::*;
use super::generic_systems::despawn_screen;
use crate::economy::nation::PlayerNation;
use crate::economy::transport::{
    TransportAdjustAllocation, TransportAllocations, TransportCapacity, TransportCommodity,
    TransportDemandSnapshot, transport_capacity, transport_demand, transport_slot,
};
use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::ui::logging::TerminalLogEvent;
use crate::ui::mode::{GameMode, switch_to_mode};

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

#[derive(Component)]
struct TransportLabel {
    commodity: TransportCommodity,
}

// Note: Fields are used in observer closure, but compiler doesn't detect this
#[derive(Component)]
#[allow(dead_code)]
struct TransportSlider {
    commodity: TransportCommodity,
    nation: Entity,
}

#[derive(Component)]
struct TransportSliderFill {
    commodity: TransportCommodity,
    kind: SliderFillKind,
}

#[derive(Component)]
struct TransportSliderBackground {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportStatsText {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportCapacityText;

#[derive(Component)]
struct TransportCapacityFill;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SliderFillKind {
    Requested,
    Granted,
}

const RESOURCE_COMMODITIES: &[TransportCommodity] = &[
    TransportCommodity::Grain,
    TransportCommodity::Fruit,
    TransportCommodity::Fiber,
    TransportCommodity::Meat,
    TransportCommodity::Timber,
    TransportCommodity::Coal,
    TransportCommodity::Iron,
    TransportCommodity::Precious,
    TransportCommodity::Oil,
];

const INDUSTRY_COMMODITIES: &[TransportCommodity] = &[
    TransportCommodity::Fabric,
    TransportCommodity::Lumber,
    TransportCommodity::Paper,
    TransportCommodity::Steel,
    TransportCommodity::Fuel,
    TransportCommodity::Clothing,
    TransportCommodity::Furniture,
    TransportCommodity::Hardware,
    TransportCommodity::Armaments,
    TransportCommodity::CannedFood,
    TransportCommodity::Horses,
];

pub struct TransportUIPlugin;

impl Plugin for TransportUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransportToolState>()
            .add_message::<TransportSelectTile>()
            .add_systems(OnEnter(GameMode::Transport), setup_transport_screen)
            .add_systems(
                OnExit(GameMode::Transport),
                despawn_screen::<TransportScreen>,
            )
            .add_systems(
                Update,
                (
                    handle_transport_selection,
                    update_transport_slider_fills,
                    update_transport_slider_backgrounds,
                    update_transport_stats_text,
                    update_transport_labels,
                    update_transport_capacity_display,
                )
                    .run_if(in_state(GameMode::Transport)),
            );
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
            let b = e.pos;
            place_writer.write(PlaceImprovement {
                a,
                b,
                kind: ImprovementKind::Road,
                engineer: None,
            });
        } else {
            tool.first = Some(e.pos);
            log.write(TerminalLogEvent {
                message: format!("Selected tile ({}, {}) for road start", e.pos.x, e.pos.y),
            });
        }
    }
}

/// Create the transport screen UI when entering the transport game mode
fn setup_transport_screen(mut commands: Commands, player: Option<Res<PlayerNation>>) {
    let Some(player) = player else {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
                TransportScreen,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Transport data unavailable: no active player nation"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.6, 0.6)),
                ));
            });
        return;
    };

    let nation = player.entity();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(24.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
            TransportScreen,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Transport Allocation"),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.95, 1.0)),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(32.0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|columns: &mut ChildSpawnerCommands| {
                    spawn_commodity_column(columns, "Resources", RESOURCE_COMMODITIES, nation);
                    spawn_commodity_column(
                        columns,
                        "Materials & Goods",
                        INDUSTRY_COMMODITIES,
                        nation,
                    );
                });

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|capacity| {
                    capacity.spawn((
                        Text::new("Capacity: 0 / 0 used"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.84, 0.92)),
                        TransportCapacityText,
                    ));

                    capacity
                        .spawn((
                            Node {
                                width: Val::Percent(60.0),
                                max_width: Val::Px(420.0),
                                height: Val::Px(18.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 1.0)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    height: Val::Percent(100.0),
                                    width: Val::Percent(0.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.35, 0.55, 0.88)),
                                TransportCapacityFill,
                            ));
                        });
                });

            parent.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    right: Val::Px(24.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::Map),
                children![(
                    Text::new("Back to Map"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                )],
            ));
        });
}

fn spawn_commodity_column(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    commodities: &[TransportCommodity],
    nation: Entity,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|column: &mut ChildSpawnerCommands| {
            column.spawn((
                Text::new(title),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.9, 1.0)),
            ));

            for &commodity in commodities {
                spawn_commodity_row(column, commodity, nation);
            }
        });
}

fn spawn_commodity_row(
    parent: &mut ChildSpawnerCommands,
    commodity: TransportCommodity,
    nation: Entity,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        },))
        .with_children(|row: &mut ChildSpawnerCommands| {
            row.spawn((
                Text::new(format!("{:?}", commodity)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.86, 0.95)),
                TransportLabel { commodity },
            ));

            // Use Bevy's standard Slider widget with observer for interaction
            row.spawn((
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(20.0),
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 1.0)),
                Hovered::default(),
                Slider::default(),
                SliderValue(0.0),
                SliderRange::new(0.0, 100.0), // Will be updated dynamically
                TransportSlider { commodity, nation },
                TransportSliderBackground { commodity },
                // Observer handles value changes
                observe(move |value_change: On<ValueChange<f32>>, mut adjust_writer: MessageWriter<TransportAdjustAllocation>| {
                    adjust_writer.write(TransportAdjustAllocation {
                        nation,
                        commodity,
                        requested: value_change.value.round() as u32,
                    });
                }),
                children![
                    // Requested fill (blue)
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.32, 0.45, 0.72)),
                        TransportSliderFill {
                            commodity,
                            kind: SliderFillKind::Requested,
                        },
                    ),
                    // Granted fill (green)
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.28, 0.76, 0.52)),
                        TransportSliderFill {
                            commodity,
                            kind: SliderFillKind::Granted,
                        },
                    ),
                    // Invisible thumb for drag interaction (required by Slider)
                    (
                        SliderThumb,
                        Node {
                            width: Val::Px(0.0),
                            height: Val::Px(0.0),
                            ..default()
                        },
                    ),
                ],
            ));

            row.spawn((
                Text::new("Requested 0 / 0 | Supply 0 | Demand 0"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.78, 0.85)),
                TransportStatsText { commodity },
            ));
        });
}

fn update_transport_slider_fills(
    player: Option<Res<PlayerNation>>,
    capacity: Res<TransportCapacity>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut slider_fills: Query<(&mut Node, &TransportSliderFill)>,
) {
    if !capacity.is_changed() && !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();
    let snapshot = transport_capacity(&capacity, nation);

    // Update visual fills based on allocation state
    for (mut node, fill) in slider_fills.iter_mut() {
        let slot = transport_slot(&allocations, nation, fill.commodity);
        let demand = transport_demand(&demand_snapshot, nation, fill.commodity);
        let scale = snapshot
            .total
            .max(slot.requested)
            .max(slot.granted)
            .max(demand.demand)
            .max(1);
        let value = match fill.kind {
            SliderFillKind::Requested => slot.requested,
            SliderFillKind::Granted => slot.granted,
        };
        let percent = (value as f32 / scale as f32 * 100.0).clamp(0.0, 100.0);
        node.width = Val::Percent(percent);
    }
}

fn update_transport_slider_backgrounds(
    player: Option<Res<PlayerNation>>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut slider_backgrounds: Query<(&mut BackgroundColor, &TransportSliderBackground)>,
) {
    if !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut background, slider) in slider_backgrounds.iter_mut() {
        let demand = transport_demand(&demand_snapshot, nation, slider.commodity);
        if demand.supply == 0 {
            background.0 = Color::srgba(0.08, 0.08, 0.1, 0.7);
        } else {
            background.0 = Color::srgba(0.12, 0.14, 0.18, 1.0);
        }
    }
}

fn update_transport_stats_text(
    player: Option<Res<PlayerNation>>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut stat_texts: Query<(&mut Text, &mut TextColor, &TransportStatsText)>,
) {
    if !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut text, mut color, stats) in stat_texts.iter_mut() {
        let slot = transport_slot(&allocations, nation, stats.commodity);
        let demand = transport_demand(&demand_snapshot, nation, stats.commodity);
        text.0 = format!(
            "Requested {} / {} | Supply {} | Demand {}",
            slot.requested, slot.granted, demand.supply, demand.demand
        );

        if demand.demand == 0 {
            color.0 = Color::srgb(0.75, 0.78, 0.85);
        } else if slot.granted >= demand.demand {
            color.0 = Color::srgb(0.55, 0.85, 0.6);
        } else {
            color.0 = Color::srgb(0.85, 0.45, 0.45);
        }
    }
}

fn update_transport_labels(
    player: Option<Res<PlayerNation>>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut labels: Query<(&mut TextColor, &TransportLabel)>,
) {
    if !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut color, label) in labels.iter_mut() {
        let demand = transport_demand(&demand_snapshot, nation, label.commodity);
        if demand.supply == 0 {
            color.0 = Color::srgb(0.5, 0.52, 0.58);
        } else {
            color.0 = Color::srgb(0.82, 0.86, 0.95);
        }
    }
}

fn update_transport_capacity_display(
    player: Option<Res<PlayerNation>>,
    capacity: Res<TransportCapacity>,
    mut capacity_text: Query<&mut Text, With<TransportCapacityText>>,
    mut capacity_fill: Query<&mut Node, With<TransportCapacityFill>>,
) {
    if !capacity.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();
    let snapshot = transport_capacity(&capacity, nation);

    if let Ok(mut text) = capacity_text.single_mut() {
        text.0 = format!("Capacity: {} / {} used", snapshot.used, snapshot.total);
    }

    if let Ok(mut node) = capacity_fill.single_mut() {
        let percent = if snapshot.total == 0 {
            0.0
        } else {
            (snapshot.used as f32 / snapshot.total as f32 * 100.0).clamp(0.0, 100.0)
        };
        node.width = Val::Percent(percent);
    }
}
