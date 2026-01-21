use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

use crate::economy::production::BuildingKind;
use crate::ui::button_style::*;
use crate::ui::city::components::BuildingButton;
use crate::ui::city::dialogs::OpenBuildingDialog;

/// Creates an observer that opens a building dialog when the button is activated
/// Only opens the dialog if the building is actually built (building_entity is Some)
pub fn open_building_on_click(building_kind: BuildingKind) -> impl Bundle {
    observe(
        move |activate: On<Activate>,
              button_query: Query<&BuildingButton>,
              mut open_dialog_writer: MessageWriter<OpenBuildingDialog>| {
            let entity = activate.entity;
            if let Ok(button) = button_query.get(entity) {
                if let Some(building_entity) = button.building_entity {
                    // Open dialog for this building
                    open_dialog_writer.write(OpenBuildingDialog {
                        building_entity,
                        building_kind,
                    });
                    info!(
                        "Opening dialog for {:?} (entity: {:?})",
                        building_kind, building_entity
                    );
                } else {
                    info!("Building button clicked but not built: {:?}", building_kind);
                    // TODO Future: Show "not built" message or construction dialog
                }
            }
        },
    )
}

/// Update building button visuals on hover (Rendering Layer)
pub fn update_building_button_visuals(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<BuildingButton>),
    >,
) {
    for (interaction, mut bg_color) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(PRESSED_BUTTON);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(HOVERED_BUTTON);
            }
            Interaction::None => {
                // Reset to built/unbuilt color (will be set by update_building_buttons)
            }
        }
    }
}
