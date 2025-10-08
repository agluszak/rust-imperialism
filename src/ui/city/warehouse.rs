use bevy::prelude::*;

use super::components::{StockpileFoodText, StockpileGoodsText, StockpileMaterialsText};

/// Update stockpile display when data changes
pub fn update_stockpile_display(
    player_nation: Option<Res<crate::economy::PlayerNation>>,
    stockpiles: Query<&crate::economy::Stockpile, Changed<crate::economy::Stockpile>>,
    mut food_text: Query<
        &mut Text,
        (
            With<StockpileFoodText>,
            Without<StockpileMaterialsText>,
            Without<StockpileGoodsText>,
        ),
    >,
    mut materials_text: Query<
        &mut Text,
        (
            With<StockpileMaterialsText>,
            Without<StockpileFoodText>,
            Without<StockpileGoodsText>,
        ),
    >,
    mut goods_text: Query<
        &mut Text,
        (
            With<StockpileGoodsText>,
            Without<StockpileFoodText>,
            Without<StockpileMaterialsText>,
        ),
    >,
) {
    use crate::economy::goods::Good;

    let Some(player) = player_nation else {
        return;
    };

    // Check if player's stockpile changed
    if let Ok(stockpile) = stockpiles.get(player.0) {
        // Update food text (available/total)
        for mut text in food_text.iter_mut() {
            text.0 = format!(
                "Food: Grain: {}/{}, Fruit: {}/{}, Livestock: {}/{}, Canned: {}/{}",
                stockpile.get_available(Good::Grain),
                stockpile.get(Good::Grain),
                stockpile.get_available(Good::Fruit),
                stockpile.get(Good::Fruit),
                stockpile.get_available(Good::Livestock),
                stockpile.get(Good::Livestock),
                stockpile.get_available(Good::CannedFood),
                stockpile.get(Good::CannedFood)
            );
        }

        // Update materials text (available/total)
        for mut text in materials_text.iter_mut() {
            text.0 = format!(
                "Materials: Wool: {}/{}, Cotton: {}/{}, Fabric: {}/{}, Paper: {}/{}",
                stockpile.get_available(Good::Wool),
                stockpile.get(Good::Wool),
                stockpile.get_available(Good::Cotton),
                stockpile.get(Good::Cotton),
                stockpile.get_available(Good::Fabric),
                stockpile.get(Good::Fabric),
                stockpile.get_available(Good::Paper),
                stockpile.get(Good::Paper)
            );
        }

        // Update goods text (available/total)
        for mut text in goods_text.iter_mut() {
            text.0 = format!(
                "Goods: Clothing: {}/{}, Furniture: {}/{}",
                stockpile.get_available(Good::Clothing),
                stockpile.get(Good::Clothing),
                stockpile.get_available(Good::Furniture),
                stockpile.get(Good::Furniture)
            );
        }
    }
}
