use bevy::prelude::*;

use crate::economy::production::BuildingKind;

use crate::ui::city::dialogs::types::{
    BuildingDialog, CloseBuildingDialog, DialogZIndexCounter, OpenBuildingDialog,
};
use crate::ui::city::dialogs::window::spawn_dialog_frame;

/// Open building dialogs (Logic Layer)
/// Spawns dialog windows when OpenBuildingDialog messages are received
pub fn open_building_dialogs(
    trigger: On<OpenBuildingDialog>,
    mut commands: Commands,
    mut z_counter: ResMut<DialogZIndexCounter>,
    city_screen: Query<Entity, With<crate::ui::city::components::CityScreen>>,
    existing_dialogs: Query<&BuildingDialog>,
) {
    let Ok(city_entity) = city_screen.single() else {
        return;
    };

    let event = trigger.event();

    // Check if dialog is already open for this building
    if existing_dialogs
        .iter()
        .any(|d| d.building_entity == event.building_entity)
    {
        info!(
            "Dialog already open for building {:?}",
            event.building_entity
        );
        return;
    }

    let z_index = z_counter.get_next();

    // Get dialog title
    let title = match event.building_kind {
        BuildingKind::TextileMill => "Textile Mill",
        BuildingKind::LumberMill => "Lumber Mill",
        BuildingKind::SteelMill => "Steel Mill",
        BuildingKind::FoodProcessingCenter => "Food Processing",
        BuildingKind::ClothingFactory => "Clothing Factory",
        BuildingKind::FurnitureFactory => "Furniture Factory",
        BuildingKind::MetalWorks => "Metal Works",
        BuildingKind::Refinery => "Refinery",
        BuildingKind::Railyard => "Railyard",
        BuildingKind::Shipyard => "Shipyard",
        BuildingKind::Capitol => "Capitol",
        BuildingKind::TradeSchool => "Trade School",
        BuildingKind::PowerPlant => "Power Plant",
    };

    // Spawn dialog frame
    let _dialog_entity = spawn_dialog_frame(
        &mut commands,
        city_entity,
        title,
        event.building_entity,
        event.building_kind,
        z_index,
    );

    info!(
        "Opened dialog for {:?} with z-index {}",
        event.building_kind, z_index
    );

    // TODO Phase 4: Populate dialog content based on building kind
}

/// Close building dialogs (Logic Layer)
/// Despawns dialog windows when CloseBuildingDialog messages are received
pub fn close_building_dialogs(
    trigger: On<CloseBuildingDialog>,
    mut commands: Commands,
    dialogs: Query<(Entity, &BuildingDialog)>,
) {
    let event = trigger.event();
    // Find and despawn the dialog (children will be despawned automatically)
    for (entity, dialog) in dialogs.iter() {
        if dialog.building_entity == event.building_entity {
            // In Bevy 0.17, despawn() on parent with with_children hierarchy
            // will clean up children automatically
            commands.entity(entity).despawn();
            info!("Closed dialog for building {:?}", event.building_entity);
        }
    }
}
