use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy_ecs_tilemap::prelude::TilePos;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::economy::nation::{Capital, PlayerNation};
use crate::economy::transport::{Depot, Port, Rails};
use crate::map::tile_pos::TilePosExt;
use crate::ui::components::MapTilemap;
use crate::ui::menu::AppState;
use crate::ui::mode::GameMode;

/// Runtime toggle for the transport network debug overlay.
#[derive(Resource, Default)]
pub struct TransportDebugSettings {
    pub enabled: bool,
}

#[derive(Resource)]
struct TransportDebugFont(Handle<Font>);

impl FromWorld for TransportDebugFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(asset_server.load("fonts/FiraSans-Bold.ttf"))
    }
}

/// Marker for debug overlay rail line visual
#[derive(Component)]
struct TransportDebugRailLine {
    #[allow(dead_code)]
    edge: (TilePos, TilePos),
}

/// Marker for debug overlay labels
#[derive(Component)]
struct TransportDebugLabel;

/// Plugin that renders transport network connectivity visualization.
pub struct TransportDebugPlugin;

impl Plugin for TransportDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransportDebugSettings>()
            .init_resource::<TransportDebugFont>()
            .add_systems(
                Update,
                (toggle_transport_debug, render_transport_debug)
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameMode::Map)),
            );
    }
}

fn toggle_transport_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<TransportDebugSettings>,
) {
    if keys.just_pressed(KeyCode::F3) {
        settings.enabled = !settings.enabled;
        info!(
            "Transport network debug overlay: {}",
            if settings.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

fn render_transport_debug(
    mut commands: Commands,
    settings: Res<TransportDebugSettings>,
    rails: Res<Rails>,
    player_nation: Option<Res<PlayerNation>>,
    capitals: Query<(Entity, &Capital)>,
    depots: Query<(Entity, &Depot)>,
    ports: Query<(Entity, &Port)>,
    font: Res<TransportDebugFont>,
    existing_lines: Query<Entity, With<TransportDebugRailLine>>,
    existing_labels: Query<Entity, With<TransportDebugLabel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Clean up existing debug visuals if disabled or if data changed
    if !settings.enabled {
        if !existing_lines.is_empty() || !existing_labels.is_empty() {
            for entity in existing_lines.iter().chain(existing_labels.iter()) {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    // Only update when settings change (not when rails change every frame)
    if !settings.is_changed() {
        return;
    }

    // Clean up old visuals
    for entity in existing_lines.iter().chain(existing_labels.iter()) {
        commands.entity(entity).despawn();
    }

    // Return early if player nation doesn't exist yet
    let Some(player_nation) = player_nation else {
        info!("Transport debug overlay enabled, but player nation not initialized yet");
        return;
    };

    // Find player's capital
    let player_capital = capitals
        .iter()
        .find(|(entity, _)| *entity == player_nation.0)
        .map(|(_, capital)| capital.0);

    let Some(capital_pos) = player_capital else {
        info!("Transport debug overlay enabled, but player capital not found");
        return;
    };

    // Build rail graph
    let graph = build_rail_graph(&rails);

    // Compute connected tiles from player's capital using BFS
    let connected_tiles = compute_connected_tiles(capital_pos, &graph);

    // Log summary
    let total_rails = rails.0.len();
    let connected_rail_count = rails
        .0
        .iter()
        .filter(|(a, b)| connected_tiles.contains(a) && connected_tiles.contains(b))
        .count();
    let player_depots = depots
        .iter()
        .filter(|(_, d)| d.owner == player_nation.0)
        .count();
    let player_ports = ports
        .iter()
        .filter(|(_, p)| p.owner == player_nation.0)
        .count();

    info!(
        "Transport debug: {}/{} rails connected, {} depots, {} ports (player)",
        connected_rail_count, total_rails, player_depots, player_ports
    );

    // Render rail segments colored by connectivity
    render_rail_segments(
        &mut commands,
        &rails,
        &connected_tiles,
        &mut meshes,
        &mut materials,
    );

    // Render depot/port labels
    render_depot_labels(&mut commands, &depots, &player_nation.0, &font);
    render_port_labels(&mut commands, &ports, &player_nation.0, &font);

    // Render connected resource summary
    render_resource_summary(&mut commands, &depots, &ports, &player_nation.0, &font);
}

/// Build adjacency list for BFS
fn build_rail_graph(rails: &Rails) -> HashMap<TilePos, Vec<TilePos>> {
    let mut graph: HashMap<TilePos, Vec<TilePos>> = HashMap::new();
    for &(a, b) in rails.0.iter() {
        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }
    graph
}

/// Compute all tiles connected to the capital via BFS
fn compute_connected_tiles(
    capital: TilePos,
    graph: &HashMap<TilePos, Vec<TilePos>>,
) -> HashSet<TilePos> {
    let mut connected = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(capital);
    connected.insert(capital);

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&current) {
            for &neighbor in neighbors {
                if !connected.contains(&neighbor) {
                    connected.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
    }

    connected
}

/// Render rail segments with colors indicating connectivity
fn render_rail_segments(
    commands: &mut Commands,
    rails: &Rails,
    connected_tiles: &HashSet<TilePos>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    const CONNECTED_RAIL_COLOR: Color = Color::srgb(0.2, 0.9, 0.2); // Bright green
    const DISCONNECTED_RAIL_COLOR: Color = Color::srgb(0.9, 0.2, 0.2); // Bright red
    const LINE_WIDTH: f32 = 3.5;
    const Z_LAYER: f32 = 3.0; // Above normal rails

    for &(a, b) in rails.0.iter() {
        // A rail segment is connected if both endpoints are reachable
        let is_connected = connected_tiles.contains(&a) && connected_tiles.contains(&b);
        let color = if is_connected {
            CONNECTED_RAIL_COLOR
        } else {
            DISCONNECTED_RAIL_COLOR
        };

        let pos_a = a.to_world_pos();
        let pos_b = b.to_world_pos();
        let center = (pos_a + pos_b) / 2.0;
        let diff = pos_b - pos_a;
        let length = diff.length();
        let angle = diff.y.atan2(diff.x);

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(length, LINE_WIDTH))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            Transform::from_translation(center.extend(Z_LAYER))
                .with_rotation(Quat::from_rotation_z(angle)),
            TransportDebugRailLine { edge: (a, b) },
            MapTilemap,
        ));
    }
}

/// Render labels for depots showing connectivity status
fn render_depot_labels(
    commands: &mut Commands,
    depots: &Query<(Entity, &Depot)>,
    player_nation: &Entity,
    font: &TransportDebugFont,
) {
    for (_, depot) in depots.iter() {
        if depot.owner != *player_nation {
            continue; // Only show player's depots
        }

        let world_pos = depot.position.to_world_pos();
        let (label, color) = if depot.connected {
            ("DEPOT ✓", Color::srgb(0.2, 0.9, 0.2))
        } else {
            ("DEPOT ✗", Color::srgb(0.9, 0.2, 0.2))
        };

        commands.spawn((
            Text2d::new(label),
            TextFont {
                font: font.0.clone(),
                font_size: 28.0,
                ..default()
            },
            TextColor(color),
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y + 25.0, 4.5))
                .with_scale(Vec3::splat(0.5)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            TransportDebugLabel,
            MapTilemap,
        ));
    }
}

/// Render labels for ports showing connectivity status
fn render_port_labels(
    commands: &mut Commands,
    ports: &Query<(Entity, &Port)>,
    player_nation: &Entity,
    font: &TransportDebugFont,
) {
    for (_, port) in ports.iter() {
        if port.owner != *player_nation {
            continue; // Only show player's ports
        }

        let world_pos = port.position.to_world_pos();
        let port_type = if port.is_river { "RIVER PORT" } else { "PORT" };
        let (label, color) = if port.connected {
            (format!("{} ✓", port_type), Color::srgb(0.2, 0.6, 1.0))
        } else {
            (format!("{} ✗", port_type), Color::srgb(0.9, 0.2, 0.2))
        };

        commands.spawn((
            Text2d::new(label),
            TextFont {
                font: font.0.clone(),
                font_size: 28.0,
                ..default()
            },
            TextColor(color),
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y - 25.0, 4.5))
                .with_scale(Vec3::splat(0.5)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            TransportDebugLabel,
            MapTilemap,
        ));
    }
}

/// Render a summary of connected resources (placeholder for future enhancement)
fn render_resource_summary(
    _commands: &mut Commands,
    depots: &Query<(Entity, &Depot)>,
    ports: &Query<(Entity, &Port)>,
    player_nation: &Entity,
    _font: &TransportDebugFont,
) {
    // Count connected vs disconnected structures
    let mut connected_depots = 0;
    let mut total_depots = 0;
    let mut connected_ports = 0;
    let mut total_ports = 0;

    for (_, depot) in depots.iter() {
        if depot.owner == *player_nation {
            total_depots += 1;
            if depot.connected {
                connected_depots += 1;
            }
        }
    }

    for (_, port) in ports.iter() {
        if port.owner == *player_nation {
            total_ports += 1;
            if port.connected {
                connected_ports += 1;
            }
        }
    }

    info!(
        "Transport network: {}/{} depots connected, {}/{} ports connected",
        connected_depots, total_depots, connected_ports, total_ports
    );
}
