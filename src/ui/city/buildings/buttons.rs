use bevy::prelude::*;

use crate::ui::button_style::*;
use crate::ui::city::components::BuildingButton;
use crate::ui::city::dialogs::OpenBuildingDialog;

/// Handle building button clicks (Input Layer)
/// Opens building dialogs when built buildings are clicked
pub fn handle_building_button_clicks(
    interactions: Query<(&Interaction, &BuildingButton), Changed<Interaction>>,
    mut open_dialog_writer: MessageWriter<OpenBuildingDialog>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(building_entity) = button.building_entity {
                // Open dialog for this building
                open_dialog_writer.write(OpenBuildingDialog {
                    building_entity,
                    building_kind: button.building_kind,
                });
                info!(
                    "Opening dialog for {:?} (entity: {:?})",
                    button.building_kind, building_entity
                );
            } else {
                info!(
                    "Building button clicked but not built: {:?}",
                    button.building_kind
                );
                // TODO Future: Show "not built" message or construction dialog
            }
        }
    }
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
