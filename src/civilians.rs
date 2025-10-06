use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::tile_pos::TilePosExt;

/// Type of civilian unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilianKind {
    Prospector, // Reveals minerals (coal/iron/gold/gems/oil)
    Miner,      // Opens & upgrades mines
    Farmer,     // Improves grain/fruit/cotton
    Rancher,    // Improves wool/livestock
    Forester,   // Improves timber
    Driller,    // Improves oil
    Engineer,   // Builds rails, depots, ports, fortifications
    Developer,  // Works in Minor Nations
}

/// Civilian unit component
#[derive(Component, Debug)]
pub struct Civilian {
    pub kind: CivilianKind,
    pub position: TilePos,
    pub owner: Entity, // Nation entity that owns this unit
    pub selected: bool,
    pub has_moved: bool, // True if unit has used its action this turn
}

/// Pending order for a civilian unit
#[derive(Component, Debug)]
pub struct CivilianOrder {
    pub target: CivilianOrderKind,
}

/// Ongoing multi-turn job for a civilian
#[derive(Component, Debug, Clone)]
pub struct CivilianJob {
    pub job_type: JobType,
    pub turns_remaining: u32,
    pub target: TilePos, // Where the job is happening
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobType {
    BuildingRail,
    BuildingDepot,
    BuildingPort,
    Mining,
    Prospecting,
    ImprovingTile,
}

/// Visual marker for civilian unit sprites
#[derive(Component)]
pub struct CivilianVisual(pub Entity); // Points to the Civilian entity

/// Marker for Engineer orders UI panel
#[derive(Component)]
pub struct EngineerOrdersPanel;

/// Marker for Build Depot button
#[derive(Component)]
pub struct BuildDepotButton;

/// Marker for Build Port button
#[derive(Component)]
pub struct BuildPortButton;

#[derive(Debug, Clone, Copy)]
pub enum CivilianOrderKind {
    BuildRail { to: TilePos }, // Build rail to adjacent tile
    BuildDepot,                // Build depot at current position
    BuildPort,                 // Build port at current position
    Move { to: TilePos },      // Move to target tile
    Prospect,                  // Reveal minerals at current tile
    Mine,                      // Upgrade mine at current tile
    ImproveTile,               // Improve resource at current tile
}

/// Message: Player selects a civilian unit
#[derive(Message, Debug, Clone, Copy)]
pub struct SelectCivilian {
    pub entity: Entity,
}

/// Handle clicks on civilian visuals to select them
pub fn handle_civilian_click(
    trigger: On<Pointer<Click>>,
    visuals: Query<&CivilianVisual>,
    mut writer: MessageWriter<SelectCivilian>,
) {
    info!(
        "handle_civilian_click triggered for entity {:?}",
        trigger.entity
    );
    if let Ok(civilian_visual) = visuals.get(trigger.entity) {
        info!(
            "Sending SelectCivilian message for entity {:?}",
            civilian_visual.0
        );
        writer.write(SelectCivilian {
            entity: civilian_visual.0,
        });
    }
}

/// Message: Player gives an order to selected civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct GiveCivilianOrder {
    pub entity: Entity,
    pub order: CivilianOrderKind,
}

pub struct CivilianPlugin;

impl Plugin for CivilianPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SelectCivilian>()
            .add_message::<GiveCivilianOrder>()
            // Selection handler runs always to react to events immediately
            .add_systems(Update, (handle_civilian_selection, handle_deselect_key))
            .add_systems(
                Update,
                (
                    handle_civilian_orders,
                    execute_engineer_orders,
                    update_engineer_orders_ui,
                    handle_order_button_clicks,
                    render_civilian_visuals,
                    update_civilian_visual_colors,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::Map)),
            );
    }
}

/// Handle Escape key to deselect all civilians
fn handle_deselect_key(keys: Res<ButtonInput<KeyCode>>, mut civilians: Query<&mut Civilian>) {
    if keys.just_pressed(KeyCode::Escape) {
        for mut civilian in civilians.iter_mut() {
            if civilian.selected {
                civilian.selected = false;
                info!("Deselected civilian via Escape key");
            }
        }
    }
}

/// Handle civilian selection events
fn handle_civilian_selection(
    mut events: MessageReader<SelectCivilian>,
    mut civilians: Query<&mut Civilian>,
) {
    let event_list: Vec<_> = events.read().collect();

    if !event_list.is_empty() {
        info!(
            "handle_civilian_selection: received {} events",
            event_list.len()
        );
    }

    // Only process if there are events
    for event in event_list {
        info!(
            "Processing SelectCivilian event for entity {:?}",
            event.entity
        );

        // Check if clicking on already-selected unit (toggle deselect)
        let is_already_selected = civilians
            .get(event.entity)
            .map(|c| c.selected)
            .unwrap_or(false);

        if is_already_selected {
            // Deselect the unit (toggle off)
            if let Ok(mut civilian) = civilians.get_mut(event.entity) {
                civilian.selected = false;
                info!("Toggled deselect for entity {:?}", event.entity);
            }
        } else {
            // Deselect all units first
            for mut civilian in civilians.iter_mut() {
                civilian.selected = false;
            }

            // Select the requested unit
            if let Ok(mut civilian) = civilians.get_mut(event.entity) {
                civilian.selected = true;
                info!(
                    "Successfully set civilian.selected = true for entity {:?}",
                    event.entity
                );
            } else {
                warn!("Failed to get civilian entity {:?}", event.entity);
            }
        }
    }
}

/// Handle civilian order events
fn handle_civilian_orders(
    mut commands: Commands,
    mut events: MessageReader<GiveCivilianOrder>,
    civilians: Query<&Civilian>,
    active_jobs: Query<&CivilianJob>,
) {
    for event in events.read() {
        if let Ok(civilian) = civilians.get(event.entity) {
            // Check if civilian has an active job
            if active_jobs.get(event.entity).is_ok() {
                info!("Civilian {:?} has active job, ignoring order", event.entity);
                continue;
            }

            // Only allow orders if unit hasn't moved this turn
            if !civilian.has_moved {
                // Add order component
                commands.entity(event.entity).insert(CivilianOrder {
                    target: event.order,
                });
            }
        }
    }
}

/// Execute Engineer orders (building infrastructure)
fn execute_engineer_orders(
    mut commands: Commands,
    mut engineers: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut improvement_writer: MessageWriter<PlaceImprovement>,
) {
    for (entity, mut civilian, order) in engineers.iter_mut() {
        // Only process Engineer units
        if civilian.kind != CivilianKind::Engineer {
            continue;
        }

        match order.target {
            CivilianOrderKind::BuildRail { to } => {
                // Send PlaceImprovement message with engineer entity
                improvement_writer.write(PlaceImprovement {
                    a: civilian.position,
                    b: to,
                    kind: ImprovementKind::Rail,
                    engineer: Some(entity),
                });
                // Move Engineer to the target tile after starting construction
                civilian.position = to;
                // Add job to lock Engineer for 3 turns
                commands.entity(entity).insert(CivilianJob {
                    job_type: JobType::BuildingRail,
                    turns_remaining: 3,
                    target: to,
                });
            }
            CivilianOrderKind::BuildDepot => {
                improvement_writer.write(PlaceImprovement {
                    a: civilian.position,
                    b: civilian.position, // Depot is single-tile
                    kind: ImprovementKind::Depot,
                    engineer: Some(entity),
                });
                // Add job to lock Engineer for 2 turns
                commands.entity(entity).insert(CivilianJob {
                    job_type: JobType::BuildingDepot,
                    turns_remaining: 2,
                    target: civilian.position,
                });
            }
            CivilianOrderKind::BuildPort => {
                improvement_writer.write(PlaceImprovement {
                    a: civilian.position,
                    b: civilian.position,
                    kind: ImprovementKind::Port,
                    engineer: Some(entity),
                });
                // Add job to lock Engineer for 2 turns
                commands.entity(entity).insert(CivilianJob {
                    job_type: JobType::BuildingPort,
                    turns_remaining: 2,
                    target: civilian.position,
                });
            }
            CivilianOrderKind::Move { to } => {
                // TODO: Implement movement with pathfinding
                civilian.position = to;
                civilian.has_moved = true;
            }
            _ => {
                // Other civilian types (Prospector, Miner, etc.) not implemented yet
            }
        }

        // Remove order after execution
        commands.entity(entity).remove::<CivilianOrder>();
    }
}

/// Reset civilian movement at start of player turn
pub fn reset_civilian_actions(mut civilians: Query<&mut Civilian>) {
    for mut civilian in civilians.iter_mut() {
        civilian.has_moved = false;
    }
}

/// Advance civilian jobs each turn
pub fn advance_civilian_jobs(
    mut commands: Commands,
    mut civilians_with_jobs: Query<(Entity, &mut CivilianJob)>,
) {
    for (entity, mut job) in civilians_with_jobs.iter_mut() {
        job.turns_remaining -= 1;

        if job.turns_remaining == 0 {
            info!("Job {:?} completed for civilian {:?}", job.job_type, entity);
            // Remove the job component
            commands.entity(entity).remove::<CivilianJob>();
        } else {
            info!(
                "Job {:?} in progress for civilian {:?}: {} turns remaining",
                job.job_type, entity, job.turns_remaining
            );
        }
    }
}

const ENGINEER_SIZE: f32 = 64.0; // Match tile size
const ENGINEER_SELECTED_COLOR: Color = Color::srgb(1.0, 0.8, 0.0); // Yellow/gold tint for selected units

/// Create/update visual sprites for civilian units
fn render_civilian_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    all_civilians: Query<(Entity, &Civilian)>,
    existing_visuals: Query<(Entity, &CivilianVisual)>,
) {
    // Remove visuals for despawned civilians
    for (visual_entity, civilian_visual) in existing_visuals.iter() {
        if all_civilians.get(civilian_visual.0).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }

    // Create visuals for new civilians
    for (civilian_entity, civilian) in all_civilians.iter() {
        // Check if visual already exists
        let visual_exists = existing_visuals
            .iter()
            .any(|(_, cv)| cv.0 == civilian_entity);

        if !visual_exists {
            let pos = civilian.position.to_world_pos();

            // Load the appropriate sprite for this civilian type
            let texture: Handle<Image> =
                asset_server.load(crate::assets::civilian_asset_path(civilian.kind));

            // Tint sprite based on selection (white = normal, yellow = selected)
            let color = if civilian.selected {
                ENGINEER_SELECTED_COLOR
            } else {
                Color::WHITE // No tint for unselected
            };

            info!(
                "Creating visual for {:?} at tile ({}, {}) -> world pos ({}, {})",
                civilian.kind, civilian.position.x, civilian.position.y, pos.x, pos.y
            );

            commands
                .spawn((
                    Sprite {
                        image: texture,
                        color,
                        custom_size: Some(Vec2::new(ENGINEER_SIZE, ENGINEER_SIZE)),
                        ..default()
                    },
                    Transform::from_translation(pos.extend(3.0)), // Above other visuals
                    CivilianVisual(civilian_entity),
                    Pickable::default(),
                ))
                .observe(handle_civilian_click);

            info!("Spawned civilian visual with transparency-enabled sprite");
        }
    }
}

/// Update civilian visual colors based on selection and position changes
fn update_civilian_visual_colors(
    civilians: Query<(Entity, &Civilian)>,
    mut visuals: Query<(&CivilianVisual, &mut Sprite, &mut Transform)>,
) {
    // Don't use Changed - just update every frame based on current state
    for (civilian_entity, civilian) in civilians.iter() {
        for (civilian_visual, mut sprite, mut transform) in visuals.iter_mut() {
            if civilian_visual.0 == civilian_entity {
                // Update color based on selection (tint yellow when selected, white when not)
                let color = if civilian.selected {
                    ENGINEER_SELECTED_COLOR
                } else {
                    Color::WHITE // No tint for unselected
                };
                sprite.color = color;

                // Update position
                let pos = civilian.position.to_world_pos();
                transform.translation = pos.extend(3.0);
            }
        }
    }
}

/// Show/hide Engineer orders UI based on selection
fn update_engineer_orders_ui(
    mut commands: Commands,
    civilians: Query<&Civilian>,
    existing_panel: Query<(Entity, &Children), With<EngineerOrdersPanel>>,
) {
    let selected_count = civilians.iter().filter(|c| c.selected).count();
    if selected_count > 0 {
        info!(
            "update_engineer_orders_ui: {} selected civilians",
            selected_count
        );
    }

    let selected_engineer = civilians
        .iter()
        .find(|c| c.selected && c.kind == CivilianKind::Engineer);

    if let Some(_engineer) = selected_engineer {
        info!("Found selected Engineer!");
        // Engineer is selected, ensure panel exists
        if existing_panel.is_empty() {
            info!("Creating Engineer orders panel");
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(16.0),
                        top: Val::Px(100.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                    EngineerOrdersPanel,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Engineer Orders"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ));

                    // Build Depot button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.3, 0.25, 1.0)),
                            BuildDepotButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Build Depot"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.95, 1.0)),
                            ));
                        });

                    // Build Port button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.25, 0.35, 1.0)),
                            BuildPortButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Build Port"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.95, 1.0)),
                            ));
                        });
                });
        }
    } else {
        // No engineer selected, remove panel and its children
        for (entity, children) in existing_panel.iter() {
            // Despawn all children first
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            // Then despawn the panel itself
            commands.entity(entity).despawn();
        }
    }
}

/// Handle button clicks in orders UI
fn handle_order_button_clicks(
    interactions: Query<
        (
            &Interaction,
            Option<&BuildDepotButton>,
            Option<&BuildPortButton>,
        ),
        Changed<Interaction>,
    >,
    selected_civilian: Query<(Entity, &Civilian), With<Civilian>>,
    mut order_writer: MessageWriter<GiveCivilianOrder>,
) {
    for (interaction, depot_button, port_button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find selected civilian
            if let Some((entity, _civilian)) = selected_civilian.iter().find(|(_, c)| c.selected) {
                if depot_button.is_some() {
                    info!("Build Depot button clicked for civilian {:?}", entity);
                    order_writer.write(GiveCivilianOrder {
                        entity,
                        order: CivilianOrderKind::BuildDepot,
                    });
                } else if port_button.is_some() {
                    info!("Build Port button clicked for civilian {:?}", entity);
                    order_writer.write(GiveCivilianOrder {
                        entity,
                        order: CivilianOrderKind::BuildPort,
                    });
                }
            }
        }
    }
}
