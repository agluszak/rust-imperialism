use bevy::prelude::*;

use super::button_style::*;
use crate::economy::{
    Allocations, Good, MARKET_RESOURCES, PlayerNation, Stockpile, Treasury, market_price,
};
use crate::ui::city::allocation_ui_unified::{
    handle_all_stepper_buttons, update_all_allocation_bars, update_all_allocation_summaries,
    update_all_stepper_displays,
};
use crate::ui::city::allocation_widgets::AllocationType;
use crate::ui::mode::{GameMode, MapModeButton};
use crate::{spawn_allocation_bar, spawn_allocation_stepper, spawn_allocation_summary};

#[derive(Component)]
pub struct MarketScreen;

#[derive(Component)]
struct MarketInventoryText {
    good: Good,
}

#[derive(Component)]
struct MarketTreasuryText;

pub struct MarketUIPlugin;

impl Plugin for MarketUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::Market), ensure_market_screen_visible)
            .add_systems(OnExit(GameMode::Market), hide_market_screen)
            .add_systems(
                Update,
                (
                    handle_all_stepper_buttons,
                    update_all_stepper_displays,
                    update_all_allocation_bars,
                    update_all_allocation_summaries,
                    update_market_treasury_text,
                    update_market_inventory_texts,
                )
                    .run_if(in_state(GameMode::Market)),
            );
    }
}

pub fn ensure_market_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<MarketScreen>>,
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
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.06, 0.92)),
            MarketScreen,
            Visibility::Visible,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Board of Trade"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.85)),
            ));

            parent.spawn((
                Text::new("Treasury available: $0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.95, 1.0)),
                MarketTreasuryText,
            ));

            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.6)),
                ))
                .with_children(|list| {
                    for &good in MARKET_RESOURCES {
                        let price = market_price(good);
                        let good_name = good.to_string();
                        list.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(18.0),
                                padding: UiRect::all(Val::Px(12.0)),
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                        ))
                        .with_children(|row| {
                            // Info column
                            row.spawn((Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                min_width: Val::Px(160.0),
                                ..default()
                            },))
                                .with_children(|info| {
                                    info.spawn((
                                        Text::new(good_name.clone()),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.95, 0.95, 0.9)),
                                    ));
                                    info.spawn((
                                        Text::new(format!("Price: ${}", price)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.75, 0.85, 1.0)),
                                    ));
                                    info.spawn((
                                        Text::new("Stockpile: 0 free / 0 total (market 0)"),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                        MarketInventoryText { good },
                                    ));
                                });

                            // Buy column
                            row.spawn((Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                min_width: Val::Px(220.0),
                                ..default()
                            },))
                                .with_children(|buy| {
                                    spawn_allocation_stepper!(
                                        buy,
                                        "Buy Orders",
                                        AllocationType::MarketBuy(good)
                                    );
                                    spawn_allocation_bar!(
                                        buy,
                                        Good::Gold,
                                        "Treasury",
                                        AllocationType::MarketBuy(good)
                                    );
                                    spawn_allocation_summary!(buy, AllocationType::MarketBuy(good));
                                });

                            // Sell column
                            row.spawn((Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                min_width: Val::Px(220.0),
                                ..default()
                            },))
                                .with_children(|sell| {
                                    spawn_allocation_stepper!(
                                        sell,
                                        "Sell Offers",
                                        AllocationType::MarketSell(good)
                                    );
                                    spawn_allocation_bar!(
                                        sell,
                                        good,
                                        "Inventory",
                                        AllocationType::MarketSell(good)
                                    );
                                    spawn_allocation_summary!(
                                        sell,
                                        AllocationType::MarketSell(good)
                                    );
                                });
                        });
                    }
                });

            // Back to Map button
            parent
                .spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(16.0),
                        right: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    MapModeButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Back to Map"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    ));
                });
        });
}

fn update_market_treasury_text(
    player: Option<Res<PlayerNation>>,
    treasuries: Query<&Treasury>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    mut texts: Query<&mut Text, With<MarketTreasuryText>>,
    treasury_changed: Query<Entity, Changed<Treasury>>,
    new_texts: Query<Entity, Added<MarketTreasuryText>>,
) {
    let Some(player) = player else {
        return;
    };

    if treasury_changed.is_empty() && allocations_changed.is_empty() && new_texts.is_empty() {
        return;
    }

    let Ok(treasury) = treasuries.get(player.0) else {
        return;
    };

    let available = treasury.available();
    let reserved = treasury.reserved();

    for mut text in texts.iter_mut() {
        text.0 = format!(
            "Treasury available: ${} (reserved ${})",
            available, reserved
        );
    }
}

fn update_market_inventory_texts(
    player: Option<Res<PlayerNation>>,
    stockpiles: Query<&Stockpile>,
    allocations: Query<&Allocations>,
    mut texts: Query<(&mut Text, &MarketInventoryText)>,
    stockpile_changed: Query<Entity, Changed<Stockpile>>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    new_texts: Query<Entity, Added<MarketInventoryText>>,
) {
    let Some(player) = player else {
        return;
    };

    if stockpile_changed.is_empty() && allocations_changed.is_empty() && new_texts.is_empty() {
        return;
    }

    let Ok(stockpile) = stockpiles.get(player.0) else {
        return;
    };

    let Ok(allocations) = allocations.get(player.0) else {
        return;
    };

    for (mut text, marker) in texts.iter_mut() {
        let total = stockpile.get(marker.good);
        let reserved = stockpile.get_reserved(marker.good);
        let available = stockpile.get_available(marker.good);
        let market_reserved = allocations.market_sell_count(marker.good) as u32;
        text.0 = format!(
            "Stockpile: {} free / {} total (reserved {}, market {})",
            available, total, reserved, market_reserved
        );
    }
}

pub fn hide_market_screen(mut roots: Query<&mut Visibility, With<MarketScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
