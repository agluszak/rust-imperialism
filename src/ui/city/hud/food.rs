use bevy::prelude::*;

use crate::economy::{PlayerNation, Workforce};
use crate::ui::city::components::{FoodDemandDisplay, FoodDemandPanel};

/// Spawn the food demand panel (right border) (Rendering Layer)
/// Takes the parent entity and commands to spawn the panel
pub fn spawn_food_demand_panel(commands: &mut Commands, parent_entity: Entity) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(10.0),
                    top: Val::Px(60.0),
                    width: Val::Px(200.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.1, 0.1, 0.95)),
                BorderColor::all(Color::srgba(0.6, 0.4, 0.4, 0.9)),
                FoodDemandPanel,
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((
                    Text::new("FOOD DEMAND"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.9, 0.9)),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Food demand breakdown (updates live)
                panel.spawn((
                    Text::new("Grain: 0\nFruit: 0\nLivestock: 0\nFish: 0"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    FoodDemandDisplay,
                ));

                // Divider
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.6, 0.5, 0.5, 0.5)),
                ));

                // Info text
                panel.spawn((
                    Text::new("Workers eat preferred\nfood. Canned food\nprevents sickness."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            });
    });
}

/// Update food demand display (Rendering Layer)
pub fn update_food_demand_display(
    player_nation: Option<Res<PlayerNation>>,
    workforce_query: Query<&Workforce>,
    mut demand_text: Query<&mut Text, With<FoodDemandDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.entity()) else {
        return;
    };

    // Calculate food demand by type based on worker preferences
    let total_workers = workforce.workers.len();
    let grain_workers = workforce
        .workers
        .iter()
        .filter(|w| w.food_preference_slot == 0)
        .count();
    let fruit_workers = workforce
        .workers
        .iter()
        .filter(|w| w.food_preference_slot == 1)
        .count();
    let livestock_workers = workforce
        .workers
        .iter()
        .filter(|w| w.food_preference_slot == 2)
        .count();
    let fish_workers = total_workers - grain_workers - fruit_workers - livestock_workers;

    for mut text in demand_text.iter_mut() {
        **text = format!(
            "Grain: {}\nFruit: {}\nLivestock: {}\nFish: {}",
            grain_workers, fruit_workers, livestock_workers, fish_workers
        );
    }
}
