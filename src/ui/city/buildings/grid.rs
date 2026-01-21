use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::Button;

use crate::economy::PlayerNation;
use crate::economy::buildings::{BuildingKind, Buildings};
use crate::ui::button_style::*;
use crate::ui::city::buildings::buttons::open_building_on_click;
use crate::ui::city::components::{BuildingButton, BuildingGrid};

/// Spawn the building grid (center area) (Rendering Layer)
/// Shows all available buildings as clickable buttons
pub fn spawn_building_grid(commands: &mut Commands, parent_entity: Entity) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(230.0),  // After labor panel
                    right: Val::Px(230.0), // Before food panel
                    top: Val::Px(140.0),   // Below warehouse HUD
                    bottom: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                BuildingGrid,
            ))
            .with_children(|grid| {
                // Title
                grid.spawn((
                    Text::new("BUILDINGS"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                // Grid container (3 columns)
                grid.spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: vec![RepeatedGridTrack::flex(3, 1.0)],
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(12.0),
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|grid_container| {
                    // Helper closure for spawning building buttons
                    let mut spawn_btn = |kind: BuildingKind, label: &str| {
                        grid_container
                            .spawn((
                                Button,
                                OldButton,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(80.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_BUTTON),
                                BorderColor::all(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                                BuildingButton {
                                    building_entity: None,
                                    building_kind: kind,
                                },
                                open_building_on_click(kind),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                    TextLayout {
                                        justify: Justify::Center,
                                        ..default()
                                    },
                                ));
                            });
                    };

                    // Production buildings
                    spawn_btn(BuildingKind::TextileMill, "Textile\nMill");
                    spawn_btn(BuildingKind::LumberMill, "Lumber\nMill");
                    spawn_btn(BuildingKind::SteelMill, "Steel\nMill");
                    spawn_btn(BuildingKind::FoodProcessingCenter, "Food\nProcessing");
                    spawn_btn(BuildingKind::ClothingFactory, "Clothing\nFactory");
                    spawn_btn(BuildingKind::FurnitureFactory, "Furniture\nFactory");
                    spawn_btn(BuildingKind::MetalWorks, "Metal\nWorks");
                    spawn_btn(BuildingKind::Refinery, "Refinery");
                    spawn_btn(BuildingKind::Railyard, "Railyard");
                    spawn_btn(BuildingKind::Shipyard, "Shipyard");

                    // Workforce buildings
                    spawn_btn(BuildingKind::Capitol, "Capitol");
                    spawn_btn(BuildingKind::TradeSchool, "Trade\nSchool");

                    // Infrastructure (future)
                    spawn_btn(BuildingKind::PowerPlant, "Power\nPlant");
                });
            });
    });
}

/// Update building buttons with actual building entities (Rendering Layer)
/// Shows which buildings are built vs. available
pub fn update_building_buttons(
    player_nation: Option<Res<PlayerNation>>,
    player_buildings_query: Query<&Buildings>,
    mut button_query: Query<(&mut BuildingButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    let Some(player) = player_nation else {
        return;
    };

    // Get player's buildings collection
    let player_entity = player.entity();
    let player_buildings = player_buildings_query.get(player_entity).ok();

    // Update button states
    for (mut button, mut bg_color, mut border_color) in button_query.iter_mut() {
        // Capitol and TradeSchool are always available (they use nation components directly)
        let always_available = matches!(
            button.building_kind,
            BuildingKind::Capitol | BuildingKind::TradeSchool
        );

        // Check if this building is built
        let is_built = always_available
            || player_buildings
                .map(|buildings| buildings.get(button.building_kind).is_some())
                .unwrap_or(false);

        if is_built {
            // Building is built - highlight button
            button.building_entity = Some(player_entity);
            *bg_color = BackgroundColor(Color::srgba(0.2, 0.3, 0.4, 1.0)); // Brighter
            *border_color = BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 1.0)); // Blue border
        } else {
            // Building not built - dim button
            button.building_entity = None;
            *bg_color = BackgroundColor(NORMAL_BUTTON);
            *border_color = BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)); // Dim border
        }
    }
}
