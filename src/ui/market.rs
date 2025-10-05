use bevy::prelude::*;

use crate::ui::mode::GameMode;
use crate::economy::{Good, PlayerNation, Treasury};

#[derive(Component)]
pub struct MarketScreen;

#[derive(Component)]
pub struct BuyClothButton;

#[derive(Component)]
pub struct SellClothButton;

pub struct MarketUIPlugin;

impl Plugin for MarketUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::Market), ensure_market_screen_visible)
            .add_systems(OnExit(GameMode::Market), hide_market_screen)
            .add_systems(Update, handle_market_buttons.run_if(in_state(GameMode::Market)));
    }
}

fn handle_market_buttons(
    mut interactions: Query<(&Interaction, Option<&BuyClothButton>, Option<&SellClothButton>), Changed<Interaction>>,
    player: Option<Res<PlayerNation>>,
    mut treasuries: Query<&mut Treasury>,
    mut stocks: Query<&mut crate::economy::Stockpile>,
) {
    if let Some(player) = player {
        for (interaction, buy, sell) in interactions.iter_mut() {
            if *interaction != Interaction::Pressed { continue; }
            if let (Ok(mut t), Ok(mut s)) = (treasuries.get_mut(player.0), stocks.get_mut(player.0)) {
                let price: i64 = 50; // fixed demo price
                if buy.is_some() {
                    if t.0 >= price {
                        t.0 -= price;
                        s.add(Good::Cloth, 1);
                    }
                } else if sell.is_some() {
                    if s.get(Good::Cloth) >= 1 {
                        let _ = s.take_up_to(Good::Cloth, 1);
                        t.0 += price;
                    }
                }
            }
        }
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
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.06, 0.92)),
            MarketScreen,
            Visibility::Visible,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Market Mode"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(1.0, 0.95, 0.85)),
            ));

            // Simple buy/sell controls
            parent.spawn((
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), ..default() },
                BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.6)),
            )).with_children(|row| {
                row.spawn((Button, Node { padding: UiRect::all(Val::Px(6.0)), ..default() }, BackgroundColor(Color::srgba(0.2,0.2,0.25,1.0)), BuyClothButton)).with_children(|b|{
                    b.spawn((Text::new("Buy 1 Cloth ($50)"), TextFont{ font_size: 16.0, ..default() }, TextColor(Color::srgb(0.9,0.9,1.0))));
                });
                row.spawn((Button, Node { padding: UiRect::all(Val::Px(6.0)), ..default() }, BackgroundColor(Color::srgba(0.2,0.2,0.25,1.0)), SellClothButton)).with_children(|b|{
                    b.spawn((Text::new("Sell 1 Cloth ($50)"), TextFont{ font_size: 16.0, ..default() }, TextColor(Color::srgb(0.9,0.9,1.0))));
                });
            });

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

pub fn hide_market_screen(mut roots: Query<&mut Visibility, With<MarketScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
