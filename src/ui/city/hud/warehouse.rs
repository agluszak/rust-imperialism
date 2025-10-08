use bevy::prelude::*;

use crate::economy::{Good, PlayerNation, Stockpile};
use crate::ui::city::components::{WarehouseHUD, WarehouseStockDisplay};

/// Spawn the warehouse HUD (top center) (Rendering Layer)
/// Takes the parent entity and commands to spawn the panel
pub fn spawn_warehouse_hud(commands: &mut Commands, parent_entity: Entity) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    left: Val::Percent(50.0),
                    width: Val::Px(600.0),
                    margin: UiRect::left(Val::Px(-300.0)), // Center it
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.9)),
                WarehouseHUD,
            ))
            .with_children(|hud| {
                // Title
                hud.spawn((
                    Text::new("WAREHOUSE"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(4.0)),
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));

                // Stock display (updates live) - compact layout
                hud.spawn((
                    Text::new(
                        "Wool: 0 | Cotton: 0 | Fabric: 0 | Grain: 0 | Fruit: 0 | Livestock: 0",
                    ),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    WarehouseStockDisplay,
                    Node {
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));
            });
    });
}

/// Update warehouse stock display (Rendering Layer)
pub fn update_warehouse_display(
    player_nation: Option<Res<PlayerNation>>,
    stockpile_query: Query<&Stockpile>,
    mut stock_text: Query<&mut Text, With<WarehouseStockDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(stockpile) = stockpile_query.get(player.0) else {
        return;
    };

    // Show key commodities in compact format
    let wool = stockpile.get(Good::Wool);
    let cotton = stockpile.get(Good::Cotton);
    let fabric = stockpile.get(Good::Fabric);
    let grain = stockpile.get(Good::Grain);
    let fruit = stockpile.get(Good::Fruit);
    let livestock = stockpile.get(Good::Livestock);
    let canned_food = stockpile.get(Good::CannedFood);
    let timber = stockpile.get(Good::Timber);
    let lumber = stockpile.get(Good::Lumber);
    let paper = stockpile.get(Good::Paper);

    for mut text in stock_text.iter_mut() {
        **text = format!(
            "Wool:{} Cotton:{} Fabric:{} | Grain:{} Fruit:{} Meat:{} Canned:{} | Wood:{} Lumber:{} Paper:{}",
            wool, cotton, fabric, grain, fruit, livestock, canned_food, timber, lumber, paper
        );
    }
}
