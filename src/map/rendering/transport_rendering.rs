use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::assets;
use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Depot, Port, Rails, Roads};
use crate::map::rendering::{MapVisual, MapVisualFor};
use crate::map::tile_pos::TilePosExt;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

/// Resource tracking the currently hovered tile
#[derive(Resource, Default)]
pub struct HoveredTile(pub Option<TilePos>);

/// Marker for rail line visual entities with edge tracking
#[derive(Component)]
pub struct RailLineVisual {
    pub edge: (TilePos, TilePos),
}

/// Marker for road line visual entities with edge tracking
#[derive(Component)]
pub struct RoadLineVisual {
    pub edge: (TilePos, TilePos),
}

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

/// Incrementally update rail line visuals to match the Rails resource
/// Only spawns/despawns changed edges instead of full redraw
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

    // Build set of existing edges
    let existing_edges: std::collections::HashSet<(TilePos, TilePos)> =
        existing.iter().map(|(_, visual)| visual.edge).collect();

    // Find edges to add (in Rails but not in existing visuals)
    for &edge in rails.0.iter() {
        if !existing_edges.contains(&edge) {
            let (a, b) = edge;
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
                RailLineVisual { edge },
            ));
        }
    }

    // Find edges to remove (in existing visuals but not in Rails)
    for (entity, visual) in existing.iter() {
        if !rails.0.contains(&visual.edge) {
            commands.entity(entity).despawn();
        }
    }
}

/// Incrementally update road line visuals to match the Roads resource
/// Only spawns/despawns changed edges instead of full redraw
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

    // Build set of existing edges
    let existing_edges: std::collections::HashSet<(TilePos, TilePos)> =
        existing.iter().map(|(_, visual)| visual.edge).collect();

    // Find edges to add (in Roads but not in existing visuals)
    for &edge in roads.0.iter() {
        if !existing_edges.contains(&edge) {
            let (a, b) = edge;
            let pos_a = a.to_world_pos();
            let pos_b = b.to_world_pos();

            let center = (pos_a + pos_b) / 2.0;
            let diff = pos_b - pos_a;
            let length = diff.length();
            let angle = diff.y.atan2(diff.x);

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(ROAD_COLOR))),
                Transform::from_translation(center.extend(0.5))
                    .with_rotation(Quat::from_rotation_z(angle)),
                RoadLineVisual { edge },
            ));
        }
    }

    // Find edges to remove (in existing visuals but not in Roads)
    for (entity, visual) in existing.iter() {
        if !roads.0.contains(&visual.edge) {
            commands.entity(entity).despawn();
        }
    }
}

/// Update depot visual colors based on connectivity
/// Uses relationship pattern for O(1) sprite lookups and automatic cleanup
fn update_depot_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    new_depots: Query<(Entity, &Depot), Added<Depot>>,
    changed_depots: Query<(Entity, &Depot, Option<&MapVisual>), Changed<Depot>>,
    mut sprites: Query<&mut Sprite>,
) {
    // Create visuals for new depots
    for (depot_entity, depot) in new_depots.iter() {
        let pos = depot.position.to_world_pos();
        let texture: Handle<Image> = asset_server.load(assets::depot_asset_path());
        let color = if depot.connected {
            DEPOT_CONNECTED_COLOR
        } else {
            DEPOT_DISCONNECTED_COLOR
        };

        commands.spawn((
            Sprite {
                image: texture,
                color,
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            Transform::from_translation(pos.extend(2.0)),
            MapVisualFor(depot_entity), // Relationship: sprite -> depot
        ));
    }

    // Update colors for changed depots (O(1) lookup via relationship)
    for (depot_entity, depot, visual) in changed_depots.iter() {
        let color = if depot.connected {
            DEPOT_CONNECTED_COLOR
        } else {
            DEPOT_DISCONNECTED_COLOR
        };

        // If depot has a visual, update its color
        if let Some(visual) = visual
            && let Ok(mut sprite) = sprites.get_mut(visual.entity())
        {
            sprite.color = color;
        } else if !new_depots.contains(depot_entity) {
            // Depot changed but has no visual (and wasn't just added) - create one
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
                MapVisualFor(depot_entity),
            ));
        }
    }
}

/// Update port visual colors based on connectivity
/// Uses relationship pattern for O(1) sprite lookups and automatic cleanup
fn update_port_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    new_ports: Query<(Entity, &Port), Added<Port>>,
    changed_ports: Query<(Entity, &Port, Option<&MapVisual>), Changed<Port>>,
    mut sprites: Query<&mut Sprite>,
) {
    // Create visuals for new ports
    for (port_entity, port) in new_ports.iter() {
        let pos = port.position.to_world_pos();
        let texture: Handle<Image> = asset_server.load(assets::port_asset_path());
        let color = if port.connected {
            PORT_CONNECTED_COLOR
        } else {
            PORT_DISCONNECTED_COLOR
        };

        commands.spawn((
            Sprite {
                image: texture,
                color,
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            Transform::from_translation(pos.extend(2.0)),
            MapVisualFor(port_entity), // Relationship: sprite -> port
        ));
    }

    // Update colors for changed ports (O(1) lookup via relationship)
    for (port_entity, port, visual) in changed_ports.iter() {
        let color = if port.connected {
            PORT_CONNECTED_COLOR
        } else {
            PORT_DISCONNECTED_COLOR
        };

        // If port has a visual, update its color
        if let Some(visual) = visual
            && let Ok(mut sprite) = sprites.get_mut(visual.entity())
        {
            sprite.color = color;
        } else if !new_ports.contains(port_entity) {
            // Port changed but has no visual (and wasn't just added) - create one
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
                MapVisualFor(port_entity),
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
