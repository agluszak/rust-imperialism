use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button};

use crate::economy::transport::TransportCommodity;
use crate::economy::{
    Allocations, Good, MARKET_RESOURCES, MarketPriceModel, MarketVolume, PlayerNation, Stockpile,
    Treasury,
};
use crate::messages::{AdjustMarketOrder, MarketInterest};
use crate::ui::button_style::*;
use crate::ui::city::allocation_ui_unified::{
    update_all_allocation_bars, update_all_allocation_summaries, update_all_stepper_displays,
};
use crate::ui::city::allocation_widgets::AllocationType;
use crate::ui::generic_systems::hide_screen;
use crate::ui::mode::{GameMode, switch_to_mode};
use crate::{spawn_allocation_bar, spawn_allocation_stepper};

#[derive(Component)]
pub struct MarketScreen;

#[derive(Component)]
struct MarketInventoryText {
    good: Good,
}

#[derive(Component)]
struct MarketTreasuryText;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MarketMode {
    Buy,
    Sell,
}

#[derive(Component)]
struct MarketModeButton {
    good: Good,
    mode: MarketMode,
}

#[derive(Component)]
struct MarketSellControls {
    good: Good,
}

#[derive(Component)]
struct BuyInterestIndicator {
    good: Good,
}

pub struct MarketUIPlugin;

impl Plugin for MarketUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::Market), ensure_market_screen_visible)
            .add_systems(OnExit(GameMode::Market), hide_screen::<MarketScreen>)
            .add_systems(
                Update,
                (
                    update_all_stepper_displays,
                    update_all_allocation_bars,
                    update_all_allocation_summaries,
                    update_market_treasury_text,
                    update_market_inventory_texts,
                    update_buy_interest_indicators,
                    update_sell_controls_visibility,
                )
                    .run_if(in_state(GameMode::Market)),
            );
    }
}

pub fn ensure_market_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<MarketScreen>>,
    asset_server: Res<AssetServer>,
    pricing: Res<MarketPriceModel>,
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
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.6)),
                ))
                .with_children(|list| {
                    for &good in MARKET_RESOURCES {
                        let price = pricing.price_for(good, MarketVolume::default());
                        let good_name = good.to_string();
                        list.spawn((
                            Node {
                                height: Val::Px(32.0),
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(6.0),
                                padding: UiRect::all(Val::Px(3.0)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                        ))
                        .with_children(|row| {
                            // Icon for the good
                            if let Some(commodity) = TransportCommodity::from_good(good) {
                                let icon_handle: Handle<Image> =
                                    asset_server.load(format!("extracted/{}", commodity.icon()));

                                row.spawn((
                                    ImageNode::new(icon_handle),
                                    Node {
                                        width: Val::Px(20.0),
                                        height: Val::Px(20.0),
                                        ..default()
                                    },
                                ));
                            }

                            // Info column (compact)
                            row.spawn((Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(1.0),
                                min_width: Val::Px(100.0),
                                ..default()
                            },))
                                .with_children(|info| {
                                    info.spawn((
                                        Text::new(format!("{} (${})  ", good_name, price)),
                                        TextFont {
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.95, 0.95, 0.9)),
                                    ));
                                    info.spawn((
                                        Text::new("0 / 0"),
                                        TextFont {
                                            font_size: 10.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.75, 0.75, 0.75)),
                                        MarketInventoryText { good },
                                    ));
                                });

                            // Mode toggle buttons
                            row.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(4.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                                .with_children(|buttons| {
                                    // Buy button
                                    buttons
                                        .spawn((
                                            Button,
                                            OldButton,
                                            Node {
                                                padding: UiRect::all(Val::Px(4.0)),
                                                ..default()
                                            },
                                            BackgroundColor(NORMAL_BUTTON),
                                            MarketModeButton {
                                                good,
                                                mode: MarketMode::Buy,
                                            },
                                        ))
                                        .observe(market_mode_button_clicked)
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new("Buy"),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                            ));
                                        });

                                    // Sell button
                                    buttons
                                        .spawn((
                                            Button,
                                            OldButton,
                                            Node {
                                                padding: UiRect::all(Val::Px(4.0)),
                                                ..default()
                                            },
                                            BackgroundColor(NORMAL_BUTTON),
                                            MarketModeButton {
                                                good,
                                                mode: MarketMode::Sell,
                                            },
                                        ))
                                        .observe(market_mode_button_clicked)
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new("Sell"),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                            ));
                                        });

                                    // Buy interest indicator
                                    buttons.spawn((
                                        Text::new(""),
                                        TextFont {
                                            font_size: 11.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.35, 0.95, 0.35)),
                                        BuyInterestIndicator { good },
                                    ));
                                });

                            // Sell controls (only visible when sell mode active)
                            row.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(4.0),
                                    align_items: AlignItems::Center,
                                    display: Display::None, // Hidden by default
                                    ..default()
                                },
                                MarketSellControls { good },
                            ))
                            .with_children(|sell| {
                                spawn_allocation_stepper!(
                                    sell,
                                    "Quantity",
                                    AllocationType::MarketSell(good)
                                );
                                spawn_allocation_bar!(
                                    sell,
                                    good,
                                    "Stock",
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
                    OldButton,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(16.0),
                        right: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    switch_to_mode(GameMode::Map),
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

    let Ok(treasury) = treasuries.get(player.entity()) else {
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

    let player_entity = player.entity();

    let Ok(stockpile) = stockpiles.get(player_entity) else {
        return;
    };

    let Ok(_allocations) = allocations.get(player_entity) else {
        return;
    };

    for (mut text, marker) in texts.iter_mut() {
        let total = stockpile.get(marker.good);
        let available = stockpile.get_available(marker.good);
        text.0 = format!("{} / {}", available, total);
    }
}

fn market_mode_button_clicked(
    trigger: On<Activate>,
    button: Query<&MarketModeButton>,
    mut writer: MessageWriter<AdjustMarketOrder>,
    player: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    mut sell_controls: Query<(&MarketSellControls, &mut Node)>,
) {
    let target = trigger.event().entity;
    let Ok(clicked_button) = button.get(target) else {
        return;
    };
    let Some(player) = player else {
        return;
    };
    let Ok(alloc) = allocations.get(player.entity()) else {
        return;
    };

    let good = clicked_button.good;
    let clicked_mode = clicked_button.mode;

    // Check current state for this good
    let has_buy = alloc.has_buy_interest(good);
    let has_sell = alloc.market_sell_count(good) > 0;

    let current_mode = if has_buy {
        Some(MarketMode::Buy)
    } else if has_sell {
        Some(MarketMode::Sell)
    } else {
        None
    };

    // Toggle behavior: clicking the active mode button turns it off
    let new_mode = if current_mode == Some(clicked_mode) {
        None // Toggle off
    } else {
        Some(clicked_mode) // Switch to this mode
    };

    // Send messages to update allocations (only if state needs to change)
    match new_mode {
        Some(MarketMode::Buy) => {
            // Express buy interest (only if not already set)
            if !has_buy {
                writer.write(AdjustMarketOrder {
                    nation: player.instance(),
                    good,
                    kind: MarketInterest::Buy,
                    requested: 1, // Non-zero = interested
                });
            }
            // Clear any sell orders when switching to buy
            if has_sell {
                writer.write(AdjustMarketOrder {
                    nation: player.instance(),
                    good,
                    kind: MarketInterest::Sell,
                    requested: 0,
                });
            }
        }
        Some(MarketMode::Sell) => {
            // Clear buy interest if switching from buy mode
            if has_buy {
                writer.write(AdjustMarketOrder {
                    nation: player.instance(),
                    good,
                    kind: MarketInterest::Buy,
                    requested: 0, // Clear interest
                });
            }
            // Sell quantity is managed by steppers
        }
        None => {
            // Clear both modes
            if has_buy {
                writer.write(AdjustMarketOrder {
                    nation: player.instance(),
                    good,
                    kind: MarketInterest::Buy,
                    requested: 0,
                });
            }
            if has_sell {
                writer.write(AdjustMarketOrder {
                    nation: player.instance(),
                    good,
                    kind: MarketInterest::Sell,
                    requested: 0,
                });
            }
        }
    }

    // Update sell controls visibility immediately
    for (ctrl, mut node) in sell_controls.iter_mut() {
        if ctrl.good != good {
            continue;
        }

        node.display = if new_mode == Some(MarketMode::Sell) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn update_buy_interest_indicators(
    player: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    mut indicators: Query<(&BuyInterestIndicator, &mut Text)>,
    new_indicators: Query<Entity, Added<BuyInterestIndicator>>,
) {
    let Some(player) = player else {
        return;
    };

    if allocations_changed.is_empty() && new_indicators.is_empty() {
        return;
    }

    let Ok(alloc) = allocations.get(player.entity()) else {
        return;
    };

    for (indicator, mut text) in indicators.iter_mut() {
        let has_buy = alloc.has_buy_interest(indicator.good);
        text.0 = if has_buy { "BID" } else { "" }.to_string();
    }
}

fn update_sell_controls_visibility(
    player: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    mut controls: Query<(&MarketSellControls, &mut Node)>,
    new_controls: Query<Entity, Added<MarketSellControls>>,
) {
    let Some(player) = player else {
        return;
    };

    if allocations_changed.is_empty() && new_controls.is_empty() {
        return;
    }

    let Ok(alloc) = allocations.get(player.entity()) else {
        return;
    };

    for (ctrl, mut node) in controls.iter_mut() {
        let has_sell = alloc.market_sell_count(ctrl.good) > 0;
        node.display = if has_sell {
            Display::Flex
        } else {
            Display::None
        };
    }
}

// Note: hide_market_screen replaced with generic hide_screen::<MarketScreen>
// See src/ui/generic_systems.rs for the generic implementation
