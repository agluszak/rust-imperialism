use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilePos;

use crate::economy::{ImprovementKind, PlaceImprovement};
use crate::tile_pos::TilePosExt;
use crate::ui::button_style::*;

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

impl JobType {
    /// Get the number of turns required for this job type
    pub fn duration(&self) -> u32 {
        match self {
            JobType::BuildingRail => 3,
            JobType::BuildingDepot => 2,
            JobType::BuildingPort => 2,
            JobType::Mining => 2,
            JobType::Prospecting => 1,
            JobType::ImprovingTile => 2,
        }
    }
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

/// Marker for resource improver orders UI panel (Farmer, Rancher, etc.)
#[derive(Component)]
pub struct ImproverOrdersPanel;

/// Marker for Improve Tile button
#[derive(Component)]
pub struct ImproveTileButton;

#[derive(Debug, Clone, Copy)]
pub enum CivilianOrderKind {
    BuildRail { to: TilePos }, // Build rail to adjacent tile
    BuildDepot,                // Build depot at current position
    BuildPort,                 // Build port at current position
    Move { to: TilePos },      // Move to target tile
    Prospect,                  // Reveal minerals at current tile (Prospector)
    Mine,                      // Upgrade mine at current tile (Miner)
    ImproveTile,               // Improve resource at current tile (Farmer/Rancher/Forester/Driller)
    BuildFarm,                 // Build farm on grain/fruit/cotton tile (Farmer)
    BuildOrchard,              // Build orchard on fruit tile (Farmer)
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

/// Message: Deselect a specific civilian
#[derive(Message, Debug, Clone, Copy)]
pub struct DeselectCivilian {
    pub entity: Entity,
}

/// Message: Deselect all civilians
#[derive(Message, Debug)]
pub struct DeselectAllCivilians;

pub struct CivilianPlugin;

impl Plugin for CivilianPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SelectCivilian>()
            .add_message::<GiveCivilianOrder>()
            .add_message::<DeselectCivilian>()
            .add_message::<DeselectAllCivilians>()
            // Selection handler runs always to react to events immediately
            .add_systems(
                Update,
                (
                    handle_civilian_selection,
                    handle_deselect_key,
                    handle_deselection,
                    handle_deselect_all,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_civilian_orders,
                    execute_move_orders,
                    execute_engineer_orders,
                    execute_prospector_orders,
                    execute_civilian_improvement_orders,
                    update_engineer_orders_ui,
                    update_improver_orders_ui,
                    handle_order_button_clicks,
                    handle_improver_button_clicks,
                    render_civilian_visuals,
                    update_civilian_visual_colors,
                )
                    .run_if(in_state(crate::ui::mode::GameMode::Map)),
            );
    }
}

/// Handle Escape key to deselect all civilians
fn handle_deselect_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<DeselectAllCivilians>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        writer.write(DeselectAllCivilians);
    }
}

/// Handle deselection of specific civilians
fn handle_deselection(
    mut events: MessageReader<DeselectCivilian>,
    mut civilians: Query<&mut Civilian>,
) {
    for event in events.read() {
        if let Ok(mut civilian) = civilians.get_mut(event.entity) {
            civilian.selected = false;
            info!("Deselected civilian {:?}", event.entity);
        }
    }
}

/// Handle deselect-all events
fn handle_deselect_all(
    mut events: MessageReader<DeselectAllCivilians>,
    mut civilians: Query<&mut Civilian>,
) {
    if !events.is_empty() {
        events.clear();
        for mut civilian in civilians.iter_mut() {
            if civilian.selected {
                civilian.selected = false;
            }
        }
        info!("Deselected all civilians via Escape key");
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

/// Check if a tile belongs to a specific nation
fn tile_owned_by_nation(
    tile_pos: TilePos,
    nation_entity: Entity,
    tile_storage: &bevy_ecs_tilemap::prelude::TileStorage,
    tile_provinces: &Query<&crate::province::TileProvince>,
    provinces: &Query<&crate::province::Province>,
) -> bool {
    if let Some(tile_entity) = tile_storage.get(&tile_pos)
        && let Ok(tile_province) = tile_provinces.get(tile_entity)
    {
        // Find the province entity with this ProvinceId
        for province in provinces.iter() {
            if province.id == tile_province.province_id {
                return province.owner == Some(nation_entity);
            }
        }
    }
    false
}

/// Execute Engineer orders (building infrastructure)
fn execute_engineer_orders(
    mut commands: Commands,
    mut engineers: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut improvement_writer: MessageWriter<PlaceImprovement>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&crate::province::TileProvince>,
    provinces: Query<&crate::province::Province>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in engineers.iter_mut() {
        // Only process Engineer units
        if civilian.kind != CivilianKind::Engineer {
            continue;
        }

        // Check territory ownership for all operations
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "Engineer cannot act at ({}, {}): tile not owned by your nation",
                    civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        match order.target {
            CivilianOrderKind::BuildRail { to } => {
                // Also check target tile ownership
                let target_owned = tile_storage_query
                    .iter()
                    .next()
                    .map(|tile_storage| {
                        tile_owned_by_nation(
                            to,
                            civilian.owner,
                            tile_storage,
                            &tile_provinces,
                            &provinces,
                        )
                    })
                    .unwrap_or(false);

                if !target_owned {
                    log_events.write(crate::ui::logging::TerminalLogEvent {
                        message: format!(
                            "Cannot build rail to ({}, {}): tile not owned by your nation",
                            to.x, to.y
                        ),
                    });
                    commands.entity(entity).remove::<CivilianOrder>();
                    continue;
                }

                // Send PlaceImprovement message with engineer entity
                improvement_writer.write(PlaceImprovement {
                    a: civilian.position,
                    b: to,
                    kind: ImprovementKind::Rail,
                    engineer: Some(entity),
                });
                // Move Engineer to the target tile after starting construction
                civilian.position = to;
                civilian.has_moved = true;
                deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                // Add job to lock Engineer
                let job_type = JobType::BuildingRail;
                commands.entity(entity).insert(CivilianJob {
                    job_type,
                    turns_remaining: job_type.duration(),
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
                civilian.has_moved = true;
                deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                // Add job to lock Engineer
                let job_type = JobType::BuildingDepot;
                commands.entity(entity).insert(CivilianJob {
                    job_type,
                    turns_remaining: job_type.duration(),
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
                civilian.has_moved = true;
                deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                // Add job to lock Engineer
                let job_type = JobType::BuildingPort;
                commands.entity(entity).insert(CivilianJob {
                    job_type,
                    turns_remaining: job_type.duration(),
                    target: civilian.position,
                });
            }
            CivilianOrderKind::Move { .. } => {
                // Move orders are handled by execute_move_orders for all civilians
            }
            _ => {
                // Other order types handled by other systems
            }
        }

        // Remove order after execution
        commands.entity(entity).remove::<CivilianOrder>();
    }
}

/// Execute Move orders for all civilian types
fn execute_move_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&crate::province::TileProvince>,
    provinces: Query<&crate::province::Province>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        if let CivilianOrderKind::Move { to } = order.target {
            // Check if target tile is owned by the civilian's nation
            let target_owned = tile_storage_query
                .iter()
                .next()
                .map(|tile_storage| {
                    tile_owned_by_nation(
                        to,
                        civilian.owner,
                        tile_storage,
                        &tile_provinces,
                        &provinces,
                    )
                })
                .unwrap_or(false);

            if !target_owned {
                log_events.write(crate::ui::logging::TerminalLogEvent {
                    message: format!(
                        "{:?} cannot move to ({}, {}): tile not owned by your nation",
                        civilian.kind, to.x, to.y
                    ),
                });
                commands.entity(entity).remove::<CivilianOrder>();
                continue;
            }

            // Simple movement: just set position (TODO: implement pathfinding)
            civilian.position = to;
            civilian.has_moved = true;
            deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after moving

            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!("{:?} moved to ({}, {})", civilian.kind, to.x, to.y),
            });

            commands.entity(entity).remove::<CivilianOrder>();
        }
    }
}

/// Execute Prospector orders (mineral discovery)
fn execute_prospector_orders(
    mut commands: Commands,
    mut prospectors: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&crate::province::TileProvince>,
    provinces: Query<&crate::province::Province>,
    mut tile_resources: Query<&mut crate::resources::TileResource>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in prospectors.iter_mut() {
        // Only process Prospector units
        if civilian.kind != CivilianKind::Prospector {
            continue;
        }

        // Check territory ownership
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "Prospector cannot act at ({}, {}): tile not owned by your nation",
                    civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        if let CivilianOrderKind::Prospect = order.target {
            // Find tile entity and check for hidden mineral
            if let Some(tile_storage) = tile_storage_query.iter().next()
                && let Some(tile_entity) = tile_storage.get(&civilian.position)
            {
                if let Ok(mut resource) = tile_resources.get_mut(tile_entity) {
                    if !resource.discovered {
                        resource.discovered = true;
                        log_events.write(crate::ui::logging::TerminalLogEvent {
                            message: format!(
                                "Prospector discovered {:?} at ({}, {})!",
                                resource.resource_type, civilian.position.x, civilian.position.y
                            ),
                        });
                        civilian.has_moved = true;
                        deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                    } else {
                        log_events.write(crate::ui::logging::TerminalLogEvent {
                            message: format!(
                                "No hidden minerals at ({}, {})",
                                civilian.position.x, civilian.position.y
                            ),
                        });
                    }
                } else {
                    log_events.write(crate::ui::logging::TerminalLogEvent {
                        message: format!(
                            "No mineral deposits at ({}, {})",
                            civilian.position.x, civilian.position.y
                        ),
                    });
                }
            }
        }

        commands.entity(entity).remove::<CivilianOrder>();
    }
}

/// Execute Farmer/Rancher/Forester/Driller orders (resource improvement)
fn execute_civilian_improvement_orders(
    mut commands: Commands,
    mut civilians: Query<(Entity, &mut Civilian, &CivilianOrder), With<Civilian>>,
    mut deselect_writer: MessageWriter<DeselectCivilian>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    tile_provinces: Query<&crate::province::TileProvince>,
    provinces: Query<&crate::province::Province>,
    tile_resources: Query<&mut crate::resources::TileResource>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for (entity, mut civilian, order) in civilians.iter_mut() {
        // Check if this is a resource-improving civilian
        let is_improver = matches!(
            civilian.kind,
            CivilianKind::Farmer
                | CivilianKind::Rancher
                | CivilianKind::Forester
                | CivilianKind::Driller
                | CivilianKind::Miner
        );

        if !is_improver {
            continue;
        }

        // Check territory ownership
        let has_territory_access = tile_storage_query
            .iter()
            .next()
            .map(|tile_storage| {
                tile_owned_by_nation(
                    civilian.position,
                    civilian.owner,
                    tile_storage,
                    &tile_provinces,
                    &provinces,
                )
            })
            .unwrap_or(false);

        if !has_territory_access {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "{:?} cannot act at ({}, {}): tile not owned by your nation",
                    civilian.kind, civilian.position.x, civilian.position.y
                ),
            });
            commands.entity(entity).remove::<CivilianOrder>();
            continue;
        }

        if let CivilianOrderKind::ImproveTile = order.target {
            // Find tile entity and validate resource
            if let Some(tile_storage) = tile_storage_query.iter().next()
                && let Some(tile_entity) = tile_storage.get(&civilian.position)
            {
                if let Ok(resource) = tile_resources.get(tile_entity) {
                    // Check if this civilian can improve this resource
                    let can_improve = match civilian.kind {
                        CivilianKind::Farmer => resource.improvable_by_farmer(),
                        CivilianKind::Rancher => resource.improvable_by_rancher(),
                        CivilianKind::Forester => resource.improvable_by_forester(),
                        CivilianKind::Miner => resource.improvable_by_miner(),
                        CivilianKind::Driller => resource.improvable_by_driller(),
                        _ => false,
                    };

                    if can_improve && resource.development < crate::resources::DevelopmentLevel::Lv3
                    {
                        // Start improvement job
                        let job_type = JobType::ImprovingTile;
                        commands.entity(entity).insert(CivilianJob {
                            job_type,
                            turns_remaining: job_type.duration(),
                            target: civilian.position,
                        });

                        log_events.write(crate::ui::logging::TerminalLogEvent {
                            message: format!(
                                "{:?} started improving {:?} at ({}, {}) - {} turns remaining",
                                civilian.kind,
                                resource.resource_type,
                                civilian.position.x,
                                civilian.position.y,
                                job_type.duration()
                            ),
                        });
                        civilian.has_moved = true;
                        deselect_writer.write(DeselectCivilian { entity }); // Auto-deselect after action
                    } else if resource.development >= crate::resources::DevelopmentLevel::Lv3 {
                        log_events.write(crate::ui::logging::TerminalLogEvent {
                            message: format!(
                                "Resource already at max development at ({}, {})",
                                civilian.position.x, civilian.position.y
                            ),
                        });
                    } else {
                        log_events.write(crate::ui::logging::TerminalLogEvent {
                            message: format!(
                                "{:?} cannot improve {:?}",
                                civilian.kind, resource.resource_type
                            ),
                        });
                    }
                } else {
                    log_events.write(crate::ui::logging::TerminalLogEvent {
                        message: format!(
                            "No improvable resource at ({}, {})",
                            civilian.position.x, civilian.position.y
                        ),
                    });
                }
            }
        }

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

/// Complete improvement jobs when they finish
pub fn complete_improvement_jobs(
    civilians_with_jobs: Query<(&Civilian, &CivilianJob)>,
    tile_storage_query: Query<&bevy_ecs_tilemap::prelude::TileStorage>,
    mut tile_resources: Query<&mut crate::resources::TileResource>,
    mut log_events: MessageWriter<crate::ui::logging::TerminalLogEvent>,
) {
    for (civilian, job) in civilians_with_jobs.iter() {
        // Only process jobs that just completed (turns_remaining == 0)
        if job.turns_remaining != 0 {
            continue;
        }

        // Only process improvement jobs
        if job.job_type != JobType::ImprovingTile {
            continue;
        }

        // Find tile entity and complete improvement
        if let Some(tile_storage) = tile_storage_query.iter().next()
            && let Some(tile_entity) = tile_storage.get(&job.target)
            && let Ok(mut resource) = tile_resources.get_mut(tile_entity)
            && resource.improve()
        {
            log_events.write(crate::ui::logging::TerminalLogEvent {
                message: format!(
                    "{:?} completed improving {:?} at ({}, {}) to level {:?}",
                    civilian.kind,
                    resource.resource_type,
                    job.target.x,
                    job.target.y,
                    resource.development
                ),
            });
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

/// Update civilian visual colors based on selection, job status, and movement
fn update_civilian_visual_colors(
    civilians: Query<(Entity, &Civilian, Option<&CivilianJob>)>,
    mut visuals: Query<(&CivilianVisual, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    // Calculate blink factor for working civilians (oscillates between 0.5 and 1.0)
    let blink_factor = (time.elapsed_secs() * 2.0).sin() * 0.25 + 0.75;

    // Don't use Changed - just update every frame based on current state
    for (civilian_entity, civilian, job) in civilians.iter() {
        for (civilian_visual, mut sprite, mut transform) in visuals.iter_mut() {
            if civilian_visual.0 == civilian_entity {
                // Determine color based on state priority:
                // 1. Selected (yellow)
                // 2. Working on job (green blink)
                // 3. Moved this turn (desaturated)
                // 4. Default (white)
                let color = if civilian.selected {
                    ENGINEER_SELECTED_COLOR
                } else if job.is_some() {
                    // Working: blink green
                    Color::srgb(0.3 * blink_factor, 1.0 * blink_factor, 0.3 * blink_factor)
                } else if civilian.has_moved {
                    // Moved: desaturated (gray)
                    Color::srgb(0.6, 0.6, 0.6)
                } else {
                    Color::WHITE // Default: no tint
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
/// Only runs when Civilian selection state changes
fn update_engineer_orders_ui(
    mut commands: Commands,
    civilians: Query<&Civilian, Changed<Civilian>>,
    all_civilians: Query<&Civilian>,
    existing_panel: Query<(Entity, &Children), With<EngineerOrdersPanel>>,
) {
    // Only run if any Civilian changed (e.g., selection state)
    if civilians.is_empty() {
        return;
    }

    let selected_engineer = all_civilians
        .iter()
        .find(|c| c.selected && c.kind == CivilianKind::Engineer);

    if let Some(_engineer) = selected_engineer {
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
                            BackgroundColor(NORMAL_BUTTON),
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
                            BackgroundColor(NORMAL_BUTTON),
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

/// Handle button clicks in Engineer orders UI
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

/// Show/hide resource improver orders UI based on selection
/// Only runs when Civilian selection state changes
fn update_improver_orders_ui(
    mut commands: Commands,
    civilians: Query<&Civilian, Changed<Civilian>>,
    all_civilians: Query<&Civilian>,
    existing_panel: Query<(Entity, &Children), With<ImproverOrdersPanel>>,
) {
    // Only run if any Civilian changed (e.g., selection state)
    if civilians.is_empty() {
        return;
    }

    let selected_improver = all_civilians.iter().find(|c| {
        c.selected
            && matches!(
                c.kind,
                CivilianKind::Farmer
                    | CivilianKind::Rancher
                    | CivilianKind::Forester
                    | CivilianKind::Miner
                    | CivilianKind::Driller
            )
    });

    if let Some(improver) = selected_improver {
        // Resource improver is selected, ensure panel exists
        if existing_panel.is_empty() {
            let panel_title = format!("{:?} Orders", improver.kind);
            info!("Creating {} orders panel", panel_title);
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
                    BackgroundColor(Color::srgba(0.1, 0.15, 0.1, 0.95)),
                    ImproverOrdersPanel,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(panel_title),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    ));

                    // Improve Tile button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            ImproveTileButton,
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Improve Tile"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 1.0)),
                            ));
                        });
                });
        }
    } else {
        // No improver selected, remove panel and its children
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

/// Handle button clicks in resource improver orders UI
fn handle_improver_button_clicks(
    interactions: Query<(&Interaction, &ImproveTileButton), Changed<Interaction>>,
    selected_civilian: Query<(Entity, &Civilian), With<Civilian>>,
    mut order_writer: MessageWriter<GiveCivilianOrder>,
) {
    for (interaction, _button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            // Find selected civilian
            if let Some((entity, civilian)) = selected_civilian.iter().find(|(_, c)| c.selected) {
                info!("Improve Tile button clicked for {:?}", civilian.kind);
                order_writer.write(GiveCivilianOrder {
                    entity,
                    order: CivilianOrderKind::ImproveTile,
                });
            }
        }
    }
}
