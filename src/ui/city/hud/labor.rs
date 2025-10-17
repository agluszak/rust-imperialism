use bevy::prelude::*;

use crate::economy::{PlayerNation, WorkerSkill, Workforce};
use crate::ui::city::components::{AvailableLaborDisplay, LaborPoolPanel, WorkforceCountDisplay};

/// Spawn the labor pool panel (left border) (Rendering Layer)
/// Takes the parent entity and commands to spawn the panel
pub fn spawn_labor_pool_panel(commands: &mut Commands, parent_entity: Entity) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(60.0),
                    width: Val::Px(200.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.15, 0.2, 0.95)),
                BorderColor::all(Color::srgba(0.4, 0.5, 0.6, 0.9)),
                LaborPoolPanel,
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((
                    Text::new("LABOR POOL"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Available labor (updates live)
                panel.spawn((
                    Text::new("Available: 0 labor"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 0.6)),
                    AvailableLaborDisplay,
                ));

                // Divider
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.5, 0.5, 0.6, 0.5)),
                ));

                // Workforce counts (updates live)
                panel.spawn((
                    Text::new("Untrained: 0\nTrained: 0\nExpert: 0"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    WorkforceCountDisplay,
                ));
            });
    });
}

/// Update available labor display (Rendering Layer)
pub fn update_labor_display(
    player_nation: Option<Res<PlayerNation>>,
    workforce_query: Query<&Workforce>,
    mut labor_text: Query<&mut Text, With<AvailableLaborDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.0) else {
        return;
    };

    let available = workforce.available_labor();

    for mut text in labor_text.iter_mut() {
        **text = format!("Available: {} labor", available);
    }
}

/// Update workforce counts display (Rendering Layer)
pub fn update_workforce_display(
    player_nation: Option<Res<PlayerNation>>,
    workforce_query: Query<&Workforce>,
    mut count_text: Query<&mut Text, With<WorkforceCountDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    let Ok(workforce) = workforce_query.get(player.0) else {
        return;
    };

    let untrained = workforce.count_by_skill(WorkerSkill::Untrained);
    let trained = workforce.count_by_skill(WorkerSkill::Trained);
    let expert = workforce.count_by_skill(WorkerSkill::Expert);

    for mut text in count_text.iter_mut() {
        **text = format!(
            "Untrained: {}\nTrained: {}\nExpert: {}",
            untrained, trained, expert
        );
    }
}
