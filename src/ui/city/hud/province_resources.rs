use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::economy::PlayerNation;
use crate::map::province::{City, Province};
use crate::resources::{ResourceType, TileResource};
use crate::ui::city::components::{ProvinceResourcesDisplay, ProvinceResourcesHUD};

/// Spawn the province resources HUD (top left) (Rendering Layer)
/// Takes the parent entity and commands to spawn the panel
pub fn spawn_province_resources_panel(commands: &mut Commands, parent_entity: Entity) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    left: Val::Px(10.0),
                    width: Val::Px(420.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.15, 0.1, 0.95)),
                BorderColor::all(Color::srgba(0.4, 0.5, 0.4, 0.9)),
                ProvinceResourcesHUD,
            ))
            .with_children(|hud| {
                // Title
                hud.spawn((
                    Text::new("PROVINCE RESOURCES"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 1.0, 0.9)),
                    Node {
                        margin: UiRect::bottom(Val::Px(4.0)),
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));

                // Resources display (updates live)
                hud.spawn((
                    Text::new("No province selected"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ProvinceResourcesDisplay,
                    Node {
                        align_self: AlignSelf::Start,
                        ..default()
                    },
                ));
            });
    });
}

/// Update province resources display (Rendering Layer)
/// Queries the player's capital city to find its province, then lists all resources in that province
pub fn update_province_resources_display(
    player_nation: Option<Res<PlayerNation>>,
    cities: Query<&City>,
    provinces: Query<&Province>,
    tile_storage_query: Query<&TileStorage>,
    tile_resources: Query<&TileResource>,
    mut display_text: Query<&mut Text, With<ProvinceResourcesDisplay>>,
) {
    let Some(_player) = player_nation else {
        return;
    };

    let Ok(mut text) = display_text.single_mut() else {
        return;
    };

    // Find player's capital city
    let player_city = cities.iter().find(|city| city.is_capital).copied();

    let Some(city) = player_city else {
        **text = "No capital city found".to_string();
        return;
    };

    // Get the province
    let province = provinces.iter().find(|p| p.id == city.province);

    let Some(province) = province else {
        **text = format!("Province {} not found", city.province.0);
        return;
    };

    // Get tile storage to look up tile entities
    let Ok(tile_storage) = tile_storage_query.single() else {
        **text = "Map not loaded".to_string();
        return;
    };

    // Collect all resources in the province
    let mut resource_counts: std::collections::HashMap<ResourceType, (u32, u32, u32)> =
        std::collections::HashMap::new();

    for tile_pos in &province.tiles {
        if let Some(tile_entity) = tile_storage.get(tile_pos)
            && let Ok(resource) = tile_resources.get(tile_entity)
            && resource.discovered
        {
            let entry = resource_counts
                .entry(resource.resource_type)
                .or_insert((0, 0, 0));
            entry.0 += 1; // Count of tiles
            let output = resource.get_output();
            entry.1 += output; // Total output
            entry.2 = entry.2.max(resource.development as u32); // Max development level
        }
    }

    // Format the output
    if resource_counts.is_empty() {
        **text = "No resources in this province".to_string();
    } else {
        let mut lines: Vec<String> = vec![format!(
            "Province {} ({} tiles)",
            province.id.0,
            province.tiles.len()
        )];

        // Sort resources by type for consistent display
        let mut sorted_resources: Vec<_> = resource_counts.iter().collect();
        sorted_resources.sort_by_key(|(res_type, _)| format!("{:?}", res_type));

        for (res_type, (count, total_output, max_dev)) in sorted_resources {
            lines.push(format!(
                "  {:?}: {} tile(s), {} output/turn (max dev: {})",
                res_type, count, total_output, max_dev
            ));
        }

        **text = lines.join("\n");
    }
}
