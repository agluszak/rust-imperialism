use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::{Depot, Port, Rails, Roads};
use crate::tile_pos::TilePosExt;

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

const RAIL_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
const ROAD_COLOR: Color = Color::srgb(0.6, 0.5, 0.4);
const DEPOT_CONNECTED_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);
const DEPOT_DISCONNECTED_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);
const PORT_CONNECTED_COLOR: Color = Color::srgb(0.2, 0.6, 0.9);
const PORT_DISCONNECTED_COLOR: Color = Color::srgb(0.6, 0.2, 0.2);

const LINE_WIDTH: f32 = 2.0;
const FACILITY_RADIUS: f32 = 6.0;

pub struct TransportRenderingPlugin;

impl Plugin for TransportRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_rails,
                render_roads,
                update_depot_visuals,
                update_port_visuals,
            )
                .run_if(in_state(crate::ui::mode::GameMode::Map))
                .run_if(in_state(crate::ui::menu::AppState::InGame)),
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
        let angle = diff.y.atan2(diff.x);

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
    all_depots: Query<(Entity, &Depot)>,
    changed_depots: Query<(Entity, &Depot), Changed<Depot>>,
    mut existing_visuals: Query<(Entity, &DepotVisual, &mut MeshMaterial2d<ColorMaterial>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
        for (_, depot_visual, mut material_handle) in existing_visuals.iter_mut() {
            if depot_visual.0 == depot_entity {
                *material_handle = MeshMaterial2d(materials.add(ColorMaterial::from_color(color)));
                found = true;
                break;
            }
        }

        // If no visual exists, create one
        if !found {
            let pos = depot.position.to_world_pos();
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(FACILITY_RADIUS))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
                Transform::from_translation(pos.extend(2.0)),
                DepotVisual(depot_entity),
            ));
        }
    }
}

/// Update port visual colors based on connectivity
fn update_port_visuals(
    mut commands: Commands,
    all_ports: Query<(Entity, &Port)>,
    changed_ports: Query<(Entity, &Port), Changed<Port>>,
    mut existing_visuals: Query<(Entity, &PortVisual, &mut MeshMaterial2d<ColorMaterial>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
        for (_, port_visual, mut material_handle) in existing_visuals.iter_mut() {
            if port_visual.0 == port_entity {
                *material_handle = MeshMaterial2d(materials.add(ColorMaterial::from_color(color)));
                found = true;
                break;
            }
        }

        // If no visual exists, create one (square for ports, circle for depots)
        if !found {
            let pos = port.position.to_world_pos();
            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(FACILITY_RADIUS * 2.0, FACILITY_RADIUS * 2.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
                Transform::from_translation(pos.extend(2.0)),
                PortVisual(port_entity),
            ));
        }
    }
}
