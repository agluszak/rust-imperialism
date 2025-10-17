use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::assets;
use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Depot, Port, Rails, Roads};
use crate::tile_pos::TilePosExt;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

/// Resource tracking the currently hovered tile
#[derive(Resource, Default)]
pub struct HoveredTile(pub Option<TilePos>);

/// Marker for rail line visual entities
#[derive(Component)]
pub struct RailLineVisual;

/// Marker for road line visual entities
#[derive(Component)]
pub struct RoadLineVisual;

/// Marker for depot visual entities
#[derive(Component)]
pub struct DepotVisual(pub Entity); // Points to the actual Depot entity

/// Marker for port visual entities
#[derive(Component)]
pub struct PortVisual(pub Entity); // Points to the actual Port entity

/// Marker for shadow rail preview visual
#[derive(Component)]
pub struct ShadowRailVisual;

const RAIL_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
const ROAD_COLOR: Color = Color::srgb(0.6, 0.5, 0.4);
const SHADOW_RAIL_COLOR: Color = Color::srgba(0.7, 0.7, 0.7, 0.4); // Semi-transparent
const DEPOT_CONNECTED_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);
const DEPOT_DISCONNECTED_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);
const PORT_CONNECTED_COLOR: Color = Color::srgb(0.2, 0.6, 0.9);
const PORT_DISCONNECTED_COLOR: Color = Color::srgb(0.6, 0.2, 0.2);

const LINE_WIDTH: f32 = 2.0;

pub struct TransportRenderingPlugin;

impl Plugin for TransportRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredTile>().add_systems(
            Update,
            (
                render_rails,
                render_roads,
                update_depot_visuals,
                update_port_visuals,
                render_shadow_rail,
            )
                .run_if(in_state(GameMode::Map))
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// Spawn/despawn rail line visuals to match the Rails resource
fn render_rails(
    mut commands: Commands,
    rails: Res<Rails>,
    existing: Query<(Entity, &RailLineVisual)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !rails.is_changed() {
        return;
    }

    // Despawn all existing rail visuals
    for (entity, _) in existing.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn new visuals for each rail edge
    for &(a, b) in rails.0.iter() {
        let pos_a = a.to_world_pos();
        let pos_b = b.to_world_pos();

        let center = (pos_a + pos_b) / 2.0;
        let diff = pos_b - pos_a;
        let length = diff.length();
        let angle = diff.y.atan2(diff.x);

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(RAIL_COLOR))),
            Transform::from_translation(center.extend(1.0))
                .with_rotation(Quat::from_rotation_z(angle)),
            RailLineVisual,
        ));
    }
}

/// Spawn/despawn road line visuals to match the Roads resource
fn render_roads(
    mut commands: Commands,
    roads: Res<Roads>,
    existing: Query<(Entity, &RoadLineVisual)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !roads.is_changed() {
        return;
    }

    // Despawn all existing road visuals
    for (entity, _) in existing.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn new visuals for each road edge
    for &(a, b) in roads.0.iter() {
        let pos_a = a.to_world_pos();
        let pos_b = b.to_world_pos();

        let center = (pos_a + pos_b) / 2.0;
        let diff = pos_b - pos_a;
        let length = diff.length();

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(ROAD_COLOR))),
            Transform::from_translation(center.extend(0.5)),
            RoadLineVisual,
        ));
    }
}

/// Update depot visual colors based on connectivity
fn update_depot_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    all_depots: Query<(Entity, &Depot)>,
    changed_depots: Query<(Entity, &Depot), Changed<Depot>>,
    mut existing_visuals: Query<(Entity, &DepotVisual, &mut Sprite)>,
) {
    // Remove visuals for despawned depots
    for (visual_entity, depot_visual, _) in existing_visuals.iter() {
        if all_depots.get(depot_visual.0).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }

    // Update colors for changed depots
    for (depot_entity, depot) in changed_depots.iter() {
        let color = if depot.connected {
            DEPOT_CONNECTED_COLOR
        } else {
            DEPOT_DISCONNECTED_COLOR
        };

        // Find and update existing visual
        let mut found = false;
        for (_, depot_visual, mut sprite) in existing_visuals.iter_mut() {
            if depot_visual.0 == depot_entity {
                sprite.color = color;
                found = true;
                break;
            }
        }

        // If no visual exists, create one with sprite
        if !found {
            let pos = depot.position.to_world_pos();
            let texture: Handle<Image> = asset_server.load(assets::depot_asset_path());
            commands.spawn((
                Sprite {
                    image: texture,
                    color,
                    custom_size: Some(Vec2::new(64.0, 64.0)),
                    ..default()
                },
                Transform::from_translation(pos.extend(2.0)),
                DepotVisual(depot_entity),
            ));
        }
    }
}

/// Update port visual colors based on connectivity
fn update_port_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    all_ports: Query<(Entity, &Port)>,
    changed_ports: Query<(Entity, &Port), Changed<Port>>,
    mut existing_visuals: Query<(Entity, &PortVisual, &mut Sprite)>,
) {
    // Remove visuals for despawned ports
    for (visual_entity, port_visual, _) in existing_visuals.iter() {
        if all_ports.get(port_visual.0).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }

    // Update colors for changed ports
    for (port_entity, port) in changed_ports.iter() {
        let color = if port.connected {
            PORT_CONNECTED_COLOR
        } else {
            PORT_DISCONNECTED_COLOR
        };

        // Find and update existing visual
        let mut found = false;
        for (_, port_visual, mut sprite) in existing_visuals.iter_mut() {
            if port_visual.0 == port_entity {
                sprite.color = color;
                found = true;
                break;
            }
        }

        // If no visual exists, create one with sprite
        if !found {
            let pos = port.position.to_world_pos();
            let texture: Handle<Image> = asset_server.load(assets::port_asset_path());
            commands.spawn((
                Sprite {
                    image: texture,
                    color,
                    custom_size: Some(Vec2::new(64.0, 64.0)),
                    ..default()
                },
                Transform::from_translation(pos.extend(2.0)),
                PortVisual(port_entity),
            ));
        }
    }
}

/// Render shadow rail preview when hovering over adjacent tiles with Engineer selected
fn render_shadow_rail(
    mut commands: Commands,
    civilians: Query<&Civilian>,
    hovered_tile: Res<HoveredTile>,
    existing_shadow: Query<Entity, With<ShadowRailVisual>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Find selected Engineer
    let selected_engineer = civilians
        .iter()
        .find(|c| c.selected && c.kind == CivilianKind::Engineer);

    // Determine if we should show shadow rail
    let should_show =
        if let (Some(engineer), Some(hovered_pos)) = (selected_engineer, hovered_tile.0) {
            // Check if hovered tile is adjacent to Engineer
            let engineer_hex = engineer.position.to_hex();
            let hovered_hex = hovered_pos.to_hex();
            engineer_hex.distance_to(hovered_hex) == 1
        } else {
            false
        };

    // Get existing shadow entity
    let has_shadow = !existing_shadow.is_empty();

    if should_show {
        let engineer = selected_engineer.unwrap();
        let hovered_pos = hovered_tile.0.unwrap();

        // Despawn old shadow if it exists
        for entity in existing_shadow.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new shadow rail
        let pos_a = engineer.position.to_world_pos();
        let pos_b = hovered_pos.to_world_pos();
        let center = (pos_a + pos_b) / 2.0;
        let diff = pos_b - pos_a;
        let length = diff.length();
        let angle = diff.y.atan2(diff.x);

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH * 1.5))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(SHADOW_RAIL_COLOR))),
            Transform::from_translation(center.extend(1.5))
                .with_rotation(Quat::from_rotation_z(angle)),
            ShadowRailVisual,
        ));
    } else if has_shadow {
        // Remove shadow rail if conditions no longer met
        for entity in existing_shadow.iter() {
            commands.entity(entity).despawn();
        }
    }
}
