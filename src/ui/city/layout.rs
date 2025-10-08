use bevy::prelude::*;

use super::components::*;
use crate::ui::button_style::*;

/// Ensure City screen is visible, creating it if needed
pub fn ensure_city_screen_visible(
    mut commands: Commands,
    mut roots: Query<&mut Visibility, With<CityScreen>>,
) {
    if let Ok(mut vis) = roots.single_mut() {
        *vis = Visibility::Visible;
        return;
    }

    // OLD CODE - No longer needed, using HUD borders and building grid instead
    // (player_stockpile, player_buildings collection removed)

    // Fullscreen city background panel
    let city_screen_entity = commands
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
            BackgroundColor(Color::srgba(0.07, 0.07, 0.1, 0.95)),
            CityScreen,
            Visibility::Visible,
        ))
        .id();

    // Spawn HUD borders
    super::hud::spawn_labor_pool_panel(&mut commands, city_screen_entity);
    super::hud::spawn_food_demand_panel(&mut commands, city_screen_entity);
    super::hud::spawn_warehouse_hud(&mut commands, city_screen_entity);

    // Spawn building grid (NEW)
    super::buildings::spawn_building_grid(&mut commands, city_screen_entity);

    // Add children to city screen
    commands.entity(city_screen_entity).with_children(|parent| {
        // Header
        parent.spawn((
            Text::new("City Overview"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
        ));

        // Return to Map button (top-right)
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
                crate::ui::mode::MapModeButton,
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
    }); // Close with_children for city_screen_entity
}

/// Hide City screen
pub fn hide_city_screen(mut roots: Query<&mut Visibility, With<CityScreen>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
