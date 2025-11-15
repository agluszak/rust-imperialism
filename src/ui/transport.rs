use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button, observe};
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::nation::PlayerNation;
use crate::economy::transport::{
    TransportAdjustAllocation, TransportAllocations, TransportCapacity, TransportCommodity,
    TransportDemandSnapshot, transport_capacity, transport_demand, transport_slot,
};
use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::ui::button_style::*;
use crate::ui::generic_systems::despawn_screen;
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
struct TransportCapacityText;

#[derive(Component)]
struct TransportCapacityFill;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportBarFillKind {
    Requested,
    Granted,
}

#[derive(Component)]
struct TransportBarFill {
    commodity: TransportCommodity,
    kind: TransportBarFillKind,
}

#[derive(Component)]
struct TransportBarBackground {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportBarText {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportIconText {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportSatisfactionFill {
    commodity: TransportCommodity,
}

#[derive(Component)]
struct TransportAdjustButton {
    commodity: TransportCommodity,
    nation: Entity,
    delta: i32,
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
                    update_transport_bar_fills,
                    update_transport_bar_backgrounds,
                    update_transport_bar_texts,
                    update_transport_icon_colors,
                    update_transport_satisfaction_bars,
                    update_transport_capacity_display,
                    update_transport_button_states,
                )
                    .run_if(in_state(GameMode::Transport)),
            );
    }
}

pub fn handle_transport_selection(
    mut ev: MessageReader<TransportSelectTile>,
    mut tool: ResMut<TransportToolState>,
    mut place_writer: MessageWriter<PlaceImprovement>,
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
            info!("Selected tile ({}, {}) for road start", e.pos.x, e.pos.y);
        }
    }
}

/// Create the transport screen UI when entering the transport game mode
fn setup_transport_screen(
    mut commands: Commands,
    player: Option<Res<PlayerNation>>,
    asset_server: Res<AssetServer>,
) {
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
                    spawn_commodity_column(
                        columns,
                        "Resources",
                        RESOURCE_COMMODITIES,
                        nation,
                        &asset_server,
                    );
                    spawn_commodity_column(
                        columns,
                        "Materials & Goods",
                        INDUSTRY_COMMODITIES,
                        nation,
                        &asset_server,
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
                OldButton,
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
    asset_server: &AssetServer,
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
                spawn_commodity_row(column, commodity, nation, asset_server);
            }
        });
}

const MAIN_BAR_WIDTH: f32 = 240.0;

fn spawn_commodity_row(
    parent: &mut ChildSpawnerCommands,
    commodity: TransportCommodity,
    nation: Entity,
    asset_server: &AssetServer,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        },))
        .with_children(|row: &mut ChildSpawnerCommands| {
            spawn_adjust_button_column(row, commodity, nation, &[-1]);

            // Load and display the commodity icon
            let icon_handle: Handle<Image> =
                asset_server.load(format!("extracted/{}", commodity.icon()));

            row.spawn((
                ImageNode::new(icon_handle),
                Node {
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.86, 0.9, 1.0)), // Default tint
                TransportIconText { commodity },
            ));

            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|bars| {
                bars.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|main| {
                    main.spawn((
                        Node {
                            width: Val::Px(MAIN_BAR_WIDTH),
                            height: Val::Px(14.0),
                            border: UiRect::all(Val::Px(1.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 1.0)),
                        TransportBarBackground { commodity },
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
                            BackgroundColor(Color::srgb(0.32, 0.45, 0.72)),
                            TransportBarFill {
                                commodity,
                                kind: TransportBarFillKind::Requested,
                            },
                        ));

                        bar.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                height: Val::Percent(100.0),
                                width: Val::Percent(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.28, 0.76, 0.52)),
                            TransportBarFill {
                                commodity,
                                kind: TransportBarFillKind::Granted,
                            },
                        ));
                    });

                    main.spawn((
                        Text::new("0 / 0"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.78, 0.85)),
                        TransportBarText { commodity },
                    ));
                });

                bars.spawn((
                    Node {
                        width: Val::Px(MAIN_BAR_WIDTH),
                        height: Val::Px(4.0),
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.7)),
                ))
                .with_children(|satisfaction| {
                    satisfaction.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.55, 0.85, 0.6)),
                        TransportSatisfactionFill { commodity },
                    ));
                });
            });

            spawn_adjust_button_column(row, commodity, nation, &[1]);
        });
}

fn spawn_adjust_button_column(
    parent: &mut ChildSpawnerCommands,
    commodity: TransportCommodity,
    nation: Entity,
    deltas: &[i32],
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|column| {
            for &delta in deltas.iter() {
                column
                    .spawn((
                        Button,
                        OldButton,
                        Node {
                            width: Val::Px(28.0),
                            height: Val::Px(22.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                        TransportAdjustButton {
                            commodity,
                            nation,
                            delta,
                        },
                        transport_adjustment_button(commodity, nation, delta),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(button_label(delta)),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                        ));
                    });
            }
        });
}

fn button_label(delta: i32) -> &'static str {
    match delta {
        -5 => "--",
        -1 => "-",
        1 => "+",
        5 => "++",
        _ if delta < 0 => "-",
        _ => "+",
    }
}

fn transport_adjustment_button(
    commodity: TransportCommodity,
    nation: Entity,
    delta: i32,
) -> impl Bundle {
    observe(
        move |_activate: On<Activate>,
              allocations: Res<TransportAllocations>,
              mut adjust_writer: MessageWriter<TransportAdjustAllocation>| {
            let slot = transport_slot(&allocations, nation, commodity);
            let current = slot.requested;
            let new_requested = adjust_requested(current, delta);

            if new_requested != current {
                adjust_writer.write(TransportAdjustAllocation {
                    nation,
                    commodity,
                    requested: new_requested,
                });
            }
        },
    )
}

fn adjust_requested(current: u32, delta: i32) -> u32 {
    if delta < 0 {
        current.saturating_sub((-delta) as u32)
    } else {
        current.saturating_add(delta as u32)
    }
}

fn update_transport_bar_fills(
    player: Option<Res<PlayerNation>>,
    capacity: Res<TransportCapacity>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut bar_fills: Query<(&mut Node, &TransportBarFill)>,
) {
    if !capacity.is_changed() && !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();
    let snapshot = transport_capacity(&capacity, nation);

    for (mut node, fill) in bar_fills.iter_mut() {
        let slot = transport_slot(&allocations, nation, fill.commodity);
        let demand = transport_demand(&demand_snapshot, nation, fill.commodity);
        let scale = snapshot
            .total
            .max(slot.requested)
            .max(slot.granted)
            .max(demand.demand)
            .max(1);
        let value = match fill.kind {
            TransportBarFillKind::Requested => slot.requested,
            TransportBarFillKind::Granted => slot.granted,
        };
        let percent = (value as f32 / scale as f32 * 100.0).clamp(0.0, 100.0);

        node.width = Val::Percent(percent);
    }
}

fn update_transport_bar_backgrounds(
    player: Option<Res<PlayerNation>>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut backgrounds: Query<(&mut BackgroundColor, &TransportBarBackground)>,
) {
    if !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut background, bar) in backgrounds.iter_mut() {
        let demand = transport_demand(&demand_snapshot, nation, bar.commodity);
        if demand.supply == 0 {
            background.0 = Color::srgba(0.08, 0.08, 0.1, 0.7);
        } else {
            background.0 = Color::srgba(0.12, 0.14, 0.18, 1.0);
        }
    }
}

fn update_transport_bar_texts(
    player: Option<Res<PlayerNation>>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut texts: Query<(&mut Text, &mut TextColor, &TransportBarText)>,
) {
    if !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut text, mut color, bar_text) in texts.iter_mut() {
        let slot = transport_slot(&allocations, nation, bar_text.commodity);
        let demand = transport_demand(&demand_snapshot, nation, bar_text.commodity);
        let target = slot.requested.max(demand.demand);
        if target == 0 {
            text.0 = format!("{} / -", slot.granted);
        } else {
            text.0 = format!("{} / {}", slot.granted, target);
        }

        if demand.demand == 0 {
            color.0 = Color::srgb(0.75, 0.78, 0.85);
        } else if slot.granted >= demand.demand {
            color.0 = Color::srgb(0.55, 0.85, 0.6);
        } else {
            color.0 = Color::srgb(0.85, 0.45, 0.45);
        }
    }
}

fn update_transport_icon_colors(
    player: Option<Res<PlayerNation>>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut icons: Query<(&mut BackgroundColor, &TransportIconText)>,
) {
    if !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut color, icon) in icons.iter_mut() {
        let demand = transport_demand(&demand_snapshot, nation, icon.commodity);
        if demand.supply == 0 {
            color.0 = Color::srgb(0.52, 0.54, 0.6);
        } else {
            color.0 = Color::srgb(0.86, 0.9, 1.0);
        }
    }
}

fn update_transport_satisfaction_bars(
    player: Option<Res<PlayerNation>>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut bars: Query<(&mut Node, &mut BackgroundColor, &TransportSatisfactionFill)>,
) {
    if !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    let Some(player) = player else {
        return;
    };
    let nation = player.entity();

    for (mut node, mut color, bar) in bars.iter_mut() {
        let slot = transport_slot(&allocations, nation, bar.commodity);
        let demand = transport_demand(&demand_snapshot, nation, bar.commodity);

        if demand.demand == 0 {
            node.width = Val::Percent(0.0);
            color.0 = Color::srgb(0.45, 0.48, 0.55);
            continue;
        }

        let ratio = (slot.granted as f32 / demand.demand as f32).clamp(0.0, 1.0);
        node.width = Val::Percent(ratio * 100.0);
        if slot.granted >= demand.demand {
            color.0 = Color::srgb(0.55, 0.85, 0.6);
        } else {
            color.0 = Color::srgb(0.85, 0.4, 0.4);
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

fn update_transport_button_states(
    capacity: Res<TransportCapacity>,
    allocations: Res<TransportAllocations>,
    demand_snapshot: Res<TransportDemandSnapshot>,
    mut buttons: Query<(
        Entity,
        &TransportAdjustButton,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
    )>,
    mut commands: Commands,
) {
    if !capacity.is_changed() && !allocations.is_changed() && !demand_snapshot.is_changed() {
        return;
    }

    for (entity, button, currently_disabled, mut bg_color) in buttons.iter_mut() {
        let slot = transport_slot(&allocations, button.nation, button.commodity);
        let demand = transport_demand(&demand_snapshot, button.nation, button.commodity);
        let cap = transport_capacity(&capacity, button.nation);

        let should_be_disabled = if button.delta < 0 {
            // Decrease button: disabled if already at 0
            slot.requested == 0
        } else {
            // Increase button: disabled if at supply limit or capacity limit
            let capacity_remaining = cap.total.saturating_sub(cap.used);
            slot.requested >= demand.supply || capacity_remaining == 0
        };

        if should_be_disabled && !currently_disabled {
            commands.entity(entity).insert(InteractionDisabled);
            bg_color.0 = Color::srgb(0.3, 0.3, 0.3);
        } else if !should_be_disabled && currently_disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
            bg_color.0 = NORMAL_BUTTON;
        }
    }
}
