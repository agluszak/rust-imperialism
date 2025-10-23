use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::HashSet;

use crate::assets;
use crate::civilians::{Civilian, CivilianKind};
use crate::economy::{Depot, Port, Rails, Roads};
use crate::map::rendering::{MapVisual, MapVisualFor};
use crate::map::tile_pos::TilePosExt;
use crate::ui::components::MapTilemap;
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

trait StructureVisual {
    fn position(&self) -> TilePos;
    fn is_connected(&self) -> bool;
}

impl StructureVisual for Depot {
    fn position(&self) -> TilePos {
        self.position
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl StructureVisual for Port {
    fn position(&self) -> TilePos {
        self.position
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

fn sync_line_visuals<Marker: Component>(
    commands: &mut Commands,
    edges: &HashSet<(TilePos, TilePos)>,
    changed: bool,
    existing: &Query<(Entity, &Marker)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    color: Color,
    z_layer: f32,
    marker_from_edge: impl Fn((TilePos, TilePos)) -> Marker,
    edge_from_marker: impl Fn(&Marker) -> (TilePos, TilePos),
) {
    if !changed {
        return;
    }

    let existing_edges: HashSet<(TilePos, TilePos)> = existing
        .iter()
        .map(|(_, marker)| edge_from_marker(marker))
        .collect();

    for &edge in edges.iter() {
        if !existing_edges.contains(&edge) {
            spawn_line_visual(
                commands,
                meshes,
                materials,
                edge,
                color,
                z_layer,
                marker_from_edge(edge),
            );
        }
    }

    for (entity, marker) in existing.iter() {
        if !edges.contains(&edge_from_marker(marker)) {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_line_visual<Marker: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    edge: (TilePos, TilePos),
    color: Color,
    z_layer: f32,
    marker: Marker,
) {
    let (a, b) = edge;
    let pos_a = a.to_world_pos();
    let pos_b = b.to_world_pos();

    let center = (pos_a + pos_b) / 2.0;
    let diff = pos_b - pos_a;
    let length = diff.length();
    let angle = diff.y.atan2(diff.x);

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
        Transform::from_translation(center.extend(z_layer))
            .with_rotation(Quat::from_rotation_z(angle)),
        marker,
        MapTilemap,
    ));
}

fn sync_structure_visuals<T: Component + StructureVisual>(
    commands: &mut Commands,
    asset_server: &AssetServer,
    new_items: &Query<(Entity, &T), Added<T>>,
    changed_items: &Query<(Entity, &T, Option<&MapVisual>), Changed<T>>,
    sprites: &mut Query<&mut Sprite>,
    asset_path: &'static str,
    connected_color: Color,
    disconnected_color: Color,
) {
    for (entity, data) in new_items.iter() {
        spawn_structure_visual(
            commands,
            asset_server,
            entity,
            data,
            asset_path,
            connected_color,
            disconnected_color,
        );
    }

    for (entity, data, visual) in changed_items.iter() {
        let color = if data.is_connected() {
            connected_color
        } else {
            disconnected_color
        };

        if let Some(visual) = visual
            && let Ok(mut sprite) = sprites.get_mut(visual.entity())
        {
            sprite.color = color;
        } else if !new_items.contains(entity) {
            spawn_structure_visual(
                commands,
                asset_server,
                entity,
                data,
                asset_path,
                connected_color,
                disconnected_color,
            );
        }
    }
}

fn spawn_structure_visual<T: Component + StructureVisual>(
    commands: &mut Commands,
    asset_server: &AssetServer,
    entity: Entity,
    data: &T,
    asset_path: &'static str,
    connected_color: Color,
    disconnected_color: Color,
) {
    let pos = data.position().to_world_pos();
    let texture: Handle<Image> = asset_server.load(asset_path);
    let color = if data.is_connected() {
        connected_color
    } else {
        disconnected_color
    };

    commands.spawn((
        Sprite {
            image: texture,
            color,
            custom_size: Some(Vec2::new(64.0, 64.0)),
            ..default()
        },
        Transform::from_translation(pos.extend(2.0)),
        MapVisualFor(entity),
        MapTilemap,
    ));
}

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
    sync_line_visuals(
        &mut commands,
        &rails.0,
        rails.is_changed(),
        &existing,
        &mut meshes,
        &mut materials,
        RAIL_COLOR,
        1.0,
        |edge| RailLineVisual { edge },
        |visual: &RailLineVisual| visual.edge,
    );
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
    sync_line_visuals(
        &mut commands,
        &roads.0,
        roads.is_changed(),
        &existing,
        &mut meshes,
        &mut materials,
        ROAD_COLOR,
        0.5,
        |edge| RoadLineVisual { edge },
        |visual: &RoadLineVisual| visual.edge,
    );
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
    sync_structure_visuals(
        &mut commands,
        &asset_server,
        &new_depots,
        &changed_depots,
        &mut sprites,
        assets::depot_asset_path(),
        DEPOT_CONNECTED_COLOR,
        DEPOT_DISCONNECTED_COLOR,
    );
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
    sync_structure_visuals(
        &mut commands,
        &asset_server,
        &new_ports,
        &changed_ports,
        &mut sprites,
        assets::port_asset_path(),
        PORT_CONNECTED_COLOR,
        PORT_DISCONNECTED_COLOR,
    );
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
            MapTilemap, // Marker for visibility control
        ));
    } else if has_shadow {
        // Remove shadow rail if conditions no longer met
        for entity in existing_shadow.iter() {
            commands.entity(entity).despawn();
        }
    }
}
