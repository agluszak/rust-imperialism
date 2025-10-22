use bevy::prelude::*;

use crate::economy::production::BuildingKind;

use super::types::{BuildingDialog, CloseBuildingDialog, DialogZIndexCounter, OpenBuildingDialog};
use super::window::spawn_dialog_frame;

/// Open building dialogs (Logic Layer)
/// Spawns dialog windows when OpenBuildingDialog messages are received
pub fn open_building_dialogs(
    mut commands: Commands,
    mut open_events: MessageReader<OpenBuildingDialog>,
    mut z_counter: ResMut<DialogZIndexCounter>,
    city_screen: Query<Entity, With<crate::ui::city::components::CityScreen>>,
    existing_dialogs: Query<&BuildingDialog>,
) {
    let Ok(city_entity) = city_screen.single() else {
        return;
    };

    for event in open_events.read() {
        // Check if dialog is already open for this building
        if existing_dialogs
            .iter()
            .any(|d| d.building_entity == event.building_entity)
        {
            info!(
                "Dialog already open for building {:?}",
                event.building_entity
            );
            continue;
        }

        let z_index = z_counter.get_next();

        // Get dialog title
        let title = match event.building_kind {
            BuildingKind::TextileMill => "Textile Mill",
            BuildingKind::ClothingFactory => "Clothing Factory",
            BuildingKind::LumberMill => "Lumber Mill",
            BuildingKind::FurnitureFactory => "Furniture Factory",
            BuildingKind::SteelMill => "Steel Mill",
            BuildingKind::MetalWorks => "Metal Works",
            BuildingKind::FoodProcessingCenter => "Food Processing",
            BuildingKind::Refinery => "Oil Refinery",
            BuildingKind::Railyard => "Railyard",
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
}

/// Close building dialogs (Logic Layer)
/// Despawns dialog windows when CloseBuildingDialog messages are received
pub fn close_building_dialogs(
    mut commands: Commands,
    mut close_events: MessageReader<CloseBuildingDialog>,
    dialogs: Query<(Entity, &BuildingDialog)>,
) {
    for event in close_events.read() {
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
}
