use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::Button;

use super::button_style::*;
use super::generic_systems::hide_screen;
use crate::economy::transport::TransportCommodity;
use crate::economy::{
    Allocations, Good, MARKET_RESOURCES, PlayerNation, Stockpile, Treasury, market_price,
};
use crate::ui::city::allocation_ui_unified::{
    update_all_allocation_bars, update_all_allocation_summaries, update_all_stepper_displays,
};
use crate::ui::city::allocation_widgets::AllocationType;
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
                )
                    .run_if(in_state(GameMode::Market)),
            );
    }
}

pub fn ensure_market_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<MarketScreen>>,
    asset_server: Res<AssetServer>,
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
                        let price = market_price(good);
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

                            // Buy column (simple toggle - no resource bar)
                            row.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(3.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                                .with_children(|buy| {
                                    spawn_allocation_stepper!(
                                        buy,
                                        "Buy Interest",
                                        AllocationType::MarketBuy(good)
                                    );
                                });

                            // Sell column (with quantity allocation)
                            row.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(4.0),
                                align_items: AlignItems::Center,
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

// Note: hide_market_screen replaced with generic hide_screen::<MarketScreen>
// See src/ui/generic_systems.rs for the generic implementation
