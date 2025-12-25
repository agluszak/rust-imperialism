use std::collections::HashMap;

use bevy::prelude::*;
use bevy::ui::widget::Button as OldButton;
use bevy::ui_widgets::{Activate, Button, observe};

use crate::diplomacy::{
    DiplomacySelection, DiplomacyState, DiplomaticOffer, DiplomaticOfferKind, DiplomaticOffers,
    DiplomaticOrder, DiplomaticOrderKind, DiplomaticRelation, ForeignAidLedger, RelationshipBand,
    resolve_offer_response,
};
use crate::economy::{Name, NationInstance, PlayerNation, Treasury};
use crate::ui::button_style::{
    AccentButton, DangerButton, NORMAL_ACCENT, NORMAL_BUTTON, NORMAL_DANGER,
};
use crate::ui::generic_systems::hide_screen;
use crate::ui::mode::{GameMode, switch_to_mode};

const PANEL_BG: Color = Color::srgba(0.08, 0.09, 0.12, 0.92);
const LIST_BG: Color = Color::srgba(0.11, 0.12, 0.15, 0.85);
const DETAIL_BG: Color = Color::srgba(0.14, 0.15, 0.18, 0.75);

#[derive(Component)]
pub struct DiplomacyScreen;

#[derive(Component)]
struct DiplomacyNationButton {
    nation: NationInstance,
}

#[derive(Component)]
struct SelectedNationNameText;

#[derive(Component)]
struct SelectedRelationText;

#[derive(Component)]
struct SelectedTreatyText;

#[derive(Component)]
struct SelectedAidText;

#[derive(Component)]
struct SelectedRelationSummaryText;

#[derive(Component)]
struct DiplomacyActionButton {
    action: DiplomaticAction,
    target: Option<NationInstance>,
}

#[derive(Component)]
struct PendingOffersContainer;

#[derive(Component)]
struct PendingOfferList;

#[derive(Clone, Copy)]
enum DiplomaticAction {
    DeclareWar,
    OfferPeace,
    Consulate,
    Embassy,
    Pact,
    Alliance,
    AidOnce(i32),
    AidLocked(i32),
    CancelAid,
}

/// Creates an observer that executes a diplomatic action when the button is activated
/// Reads current selection from DiplomacySelection resource and player NationInstance from PlayerNation
fn execute_diplomatic_action(action: DiplomaticAction) -> impl Bundle {
    observe(
        move |_activate: On<Activate>,
              selection: Res<DiplomacySelection>,
              player: Option<Res<PlayerNation>>,
              mut orders: MessageWriter<DiplomaticOrder>| {
            let Some(selected) = selection.selected else {
                return;
            };

            let player_instance = match player {
                Some(p) => p.instance(),
                None => return,
            };

            let order = match action {
                DiplomaticAction::DeclareWar => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::DeclareWar,
                },
                DiplomaticAction::OfferPeace => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::OfferPeace,
                },
                DiplomaticAction::Consulate => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::EstablishConsulate,
                },
                DiplomaticAction::Embassy => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::OpenEmbassy,
                },
                DiplomaticAction::Pact => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::SignNonAggressionPact,
                },
                DiplomaticAction::Alliance => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::FormAlliance,
                },
                DiplomaticAction::AidOnce(amount) => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::SendAid {
                        amount,
                        locked: false,
                    },
                },
                DiplomaticAction::AidLocked(amount) => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::SendAid {
                        amount,
                        locked: true,
                    },
                },
                DiplomaticAction::CancelAid => DiplomaticOrder {
                    actor: player_instance,
                    target: selected,
                    kind: DiplomaticOrderKind::CancelAid,
                },
            };

            orders.write(order);
        },
    )
}

pub struct DiplomacyUIPlugin;

impl Plugin for DiplomacyUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameMode::Diplomacy), setup_diplomacy_screen)
            .add_systems(OnExit(GameMode::Diplomacy), hide_screen::<DiplomacyScreen>);

        // Add systems individually since ParamSet breaks .chain()
        // Systems will run in parallel where possible (Bevy handles scheduling)
        app.add_systems(
            Update,
            (
                ensure_selection_valid,
                update_nation_buttons,
                update_action_buttons,
                update_pending_offers,
            )
                .run_if(in_state(GameMode::Diplomacy)),
        );

        // Add detail panel update separately - can't use run_if with ParamSet
        // State check is done inside the system
        app.add_systems(Update, update_detail_panel);
    }
}

fn setup_diplomacy_screen(
    mut commands: Commands,
    mut screen_visibility: Query<&mut Visibility, With<DiplomacyScreen>>,
    nations: Query<(NationInstance, &Name)>,
    player: Option<Res<PlayerNation>>,
    mut selection: ResMut<DiplomacySelection>,
) {
    if let Ok(mut visibility) = screen_visibility.single_mut() {
        *visibility = Visibility::Visible;
        return;
    }

    let player_instance = player.as_ref().map(|p| p.instance());

    let mut foreign_nations: Vec<(NationInstance, String)> = nations
        .iter()
        .filter_map(|(instance, name)| {
            if Some(instance) == player_instance {
                None
            } else {
                Some((instance, name.0.clone()))
            }
        })
        .collect();
    foreign_nations.sort_by(|a, b| a.1.cmp(&b.1));

    if selection
        .selected
        .map(|sel| foreign_nations.iter().any(|(inst, _)| *inst == sel))
        .unwrap_or(false)
    {
        // Keep existing selection
    } else {
        selection.selected = foreign_nations.first().map(|(inst, _)| *inst);
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DiplomacyScreen,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Foreign Office"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.92, 0.86)),
            ));

            root.spawn((
                Text::new("Review relations and issue diplomatic overtures."),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.85, 0.9)),
            ));

            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                height: Val::Percent(100.0),
                ..default()
            },))
                .with_children(|body| {
                    // Nation list
                    body.spawn((
                        Node {
                            width: Val::Percent(32.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(LIST_BG),
                    ))
                    .with_children(|list| {
                        list.spawn((
                            Text::new("Nations"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.92, 0.98)),
                        ));

                        for (nation_instance, name) in &foreign_nations {
                            let nation_copy = *nation_instance;
                            list.spawn((
                                Button,
                                OldButton,
                                Node {
                                    padding: UiRect::all(Val::Px(8.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_BUTTON),
                                DiplomacyNationButton { nation: nation_copy },
                            ))
                            .observe(
                                move |_trigger: On<Pointer<Click>>,
                                      mut selection: ResMut<DiplomacySelection>| {
                                    selection.selected = Some(nation_copy);
                                },
                            )
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(name.clone()),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.9, 1.0)),
                                ));
                            });
                        }

                        if foreign_nations.is_empty() {
                            list.spawn((
                                Text::new("No foreign nations discovered yet."),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.75, 0.75, 0.8)),
                            ));
                        }
                    });

                    // Detail panel
                    body.spawn((
                        Node {
                            width: Val::Percent(68.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(12.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(DETAIL_BG),
                    ))
                    .with_children(|detail| {
                        detail.spawn((
                            Text::new("Select a nation from the list."),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.96, 1.0)),
                            SelectedNationNameText,
                        ));

                        detail.spawn((
                            Text::new("Relationship: --"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.88, 0.95)),
                            SelectedRelationText,
                        ));

                        detail.spawn((
                            Text::new("Standing: unknown"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.82, 0.88)),
                            SelectedRelationSummaryText,
                        ));

                        detail.spawn((
                            Text::new("Treaties: none"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.82, 0.88)),
                            SelectedTreatyText,
                        ));

                        detail.spawn((
                            Text::new("Locked aid: none"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.82, 0.88)),
                            SelectedAidText,
                        ));

                        // War / peace actions
                        detail
                            .spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },))
                            .with_children(|row| {
                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_DANGER),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::DeclareWar,
                                        target: None,
                                    },
                                    DangerButton,
                                    execute_diplomatic_action(DiplomaticAction::DeclareWar),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Declare War".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::OfferPeace,
                                        target: None,
                                    },
                                    execute_diplomatic_action(DiplomaticAction::OfferPeace),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Offer Peace".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });
                            });

                        // Diplomatic upgrades
                        detail
                            .spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },))
                            .with_children(|row| {
                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Consulate,
                                        target: None,
                                    },
                                    AccentButton,
                                    execute_diplomatic_action(DiplomaticAction::Consulate),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Open Consulate ($500)".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Embassy,
                                        target: None,
                                    },
                                    AccentButton,
                                    execute_diplomatic_action(DiplomaticAction::Embassy),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Open Embassy ($5,000)".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Pact,
                                        target: None,
                                    },
                                    execute_diplomatic_action(DiplomaticAction::Pact),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Non-Aggression Pact".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Alliance,
                                        target: None,
                                    },
                                    AccentButton,
                                    execute_diplomatic_action(DiplomaticAction::Alliance),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Form Alliance".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });
                            });

                        // Aid controls
                        detail
                            .spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },))
                            .with_children(|row| {
                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::AidOnce(500),
                                        target: None,
                                    },
                                    AccentButton,
                                    execute_diplomatic_action(DiplomaticAction::AidOnce(500)),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Send $500 Aid".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::AidLocked(500),
                                        target: None,
                                    },
                                    AccentButton,
                                    execute_diplomatic_action(DiplomaticAction::AidLocked(500)),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Lock $500 Aid".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });

                                row.spawn((
                                    Button,
                                    OldButton,
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::CancelAid,
                                        target: None,
                                    },
                                    execute_diplomatic_action(DiplomaticAction::CancelAid),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Cancel Locked Aid".to_string()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                    ));
                                });
                            });
                    });
                });

            // Back button
            root.spawn((
                Button,
                OldButton,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                switch_to_mode(GameMode::Map),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Back to Map"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 1.0)),
                ));
            });

            root
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(LIST_BG),
                    PendingOffersContainer,
                ))
                .with_children(|offers| {
                    offers.spawn((
                        Text::new("Pending Offers"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                    ));
                    offers.spawn((
                        Text::new("Foreign governments will occasionally send overtures that must be answered before the next turn."),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.8, 0.85)),
                    ));
                    offers
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                            PendingOfferList,
                        ))
                        .with_children(|list| {
                            list.spawn((
                                Text::new("No pending offers."),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.83, 0.9)),
                            ));
                        });
                });
        });
}

fn ensure_selection_valid(
    mut selection: ResMut<DiplomacySelection>,
    player: Option<Res<PlayerNation>>,
    nations: Query<NationInstance>,
) {
    let player_instance = player.as_ref().map(|p| p.instance());
    let available: Vec<NationInstance> = nations
        .iter()
        .filter(|inst| Some(*inst) != player_instance)
        .collect();

    if let Some(sel) = selection.selected
        && !available.contains(&sel)
    {
        selection.selected = None;
    }

    if selection.selected.is_none() {
        selection.selected = available.first().copied();
    }
}

fn update_nation_buttons(
    selection: Res<DiplomacySelection>,
    state: Res<DiplomacyState>,
    player: Option<Res<PlayerNation>>,
    names: Query<(NationInstance, &Name)>,
    mut buttons: Query<
        (&DiplomacyNationButton, &mut Text, &mut BackgroundColor),
        (
            Without<SelectedNationNameText>,
            Without<SelectedRelationText>,
            Without<SelectedRelationSummaryText>,
            Without<SelectedTreatyText>,
            Without<SelectedAidText>,
        ),
    >,
) {
    let player_instance = player.as_ref().map(|p| p.instance());

    // Build name lookup by entity
    let name_map: HashMap<Entity, String> = names
        .iter()
        .map(|(inst, n)| (inst.entity(), n.0.clone()))
        .collect();

    for (button, mut text, mut color) in buttons.iter_mut() {
        let label = name_map
            .get(&button.nation.entity())
            .cloned()
            .unwrap_or_else(|| format!("Nation {:?}", button.nation.entity()));

        let mut relation_line = label.clone();
        if let Some(player_inst) = player_instance
            && let Some(relation) = state.relation(player_inst, button.nation)
        {
            relation_line = format!(
                "{} — {} ({})",
                label,
                relation.score,
                relation.band().label()
            );
        }

        text.0 = relation_line;

        if selection.selected == Some(button.nation) {
            *color = BackgroundColor(NORMAL_ACCENT);
        } else {
            *color = BackgroundColor(NORMAL_BUTTON);
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_detail_panel(
    screen_query: Query<&Visibility, With<DiplomacyScreen>>,
    selection: Res<DiplomacySelection>,
    state: Res<DiplomacyState>,
    ledger: Res<ForeignAidLedger>,
    player: Option<Res<PlayerNation>>,
    names: Query<(NationInstance, &Name)>,
    mut text_queries: ParamSet<(
        Query<&mut Text, (With<SelectedNationNameText>, Without<DiplomacyNationButton>)>,
        Query<&mut Text, (With<SelectedRelationText>, Without<DiplomacyNationButton>)>,
        Query<
            &mut Text,
            (
                With<SelectedRelationSummaryText>,
                Without<DiplomacyNationButton>,
            ),
        >,
        Query<&mut Text, (With<SelectedTreatyText>, Without<DiplomacyNationButton>)>,
        Query<&mut Text, (With<SelectedAidText>, Without<DiplomacyNationButton>)>,
    )>,
) {
    // Early exit if screen is not visible (not in Diplomacy mode)
    let Ok(visibility) = screen_query.single() else {
        return;
    };
    if *visibility != Visibility::Visible {
        return;
    }
    let Some(selected) = selection.selected else {
        if let Ok(mut text) = text_queries.p0().single_mut() {
            text.0 = "No foreign nations selected".to_string();
        }
        if let Ok(mut text) = text_queries.p1().single_mut() {
            text.0 = "Relationship: --".to_string();
        }
        if let Ok(mut text) = text_queries.p2().single_mut() {
            text.0 = "Standing: unknown".to_string();
        }
        if let Ok(mut text) = text_queries.p3().single_mut() {
            text.0 = "Treaties: none".to_string();
        }
        if let Ok(mut text) = text_queries.p4().single_mut() {
            text.0 = "Locked aid: none".to_string();
        }
        return;
    };

    let player_instance = player.as_ref().map(|p| p.instance());

    let selected_name = names
        .iter()
        .find(|(e, _)| *e == selected.entity())
        .map(|(_, name)| name.0.clone())
        .unwrap_or_else(|| format!("Nation {:?}", selected.entity()));

    if let Ok(mut text) = text_queries.p0().single_mut() {
        text.0 = selected_name.clone();
    }

    let relation = player_instance.and_then(|pid| state.relation(pid, selected));
    if let Ok(mut text) = text_queries.p1().single_mut() {
        if let Some(relation) = relation {
            text.0 = format!(
                "Relationship: {} ({})",
                relation.score,
                relation.band().label()
            );
        } else {
            text.0 = "Relationship: unknown".to_string();
        }
    }

    if let Ok(mut text) = text_queries.p2().single_mut() {
        if let Some(relation) = relation {
            text.0 = format!("Standing: {}", relation_summary(relation));
        } else {
            text.0 = "Standing: unknown".to_string();
        }
    }

    if let Ok(mut text) = text_queries.p3().single_mut() {
        if let Some(relation) = relation {
            let mut flags = Vec::new();
            if relation.treaty.at_war {
                flags.push("At war");
            } else {
                flags.push("At peace");
            }
            if relation.treaty.consulate {
                flags.push("Consulate");
            }
            if relation.treaty.embassy {
                flags.push("Embassy");
            }
            if relation.treaty.non_aggression_pact {
                flags.push("Pact");
            }
            if relation.treaty.alliance {
                flags.push("Alliance");
            }
            text.0 = format!("Treaties: {}", flags.join(", "));
        } else {
            text.0 = "Treaties: none".to_string();
        }
    }

    if let Ok(mut text) = text_queries.p4().single_mut() {
        if let Some(player_inst) = player_instance {
            if let Some(grant) = ledger
                .all()
                .iter()
                .find(|g| g.from == player_inst && g.to == selected)
            {
                text.0 = format!("Locked aid: ${} per turn", grant.amount);
            } else {
                text.0 = "Locked aid: none".to_string();
            }
        } else {
            text.0 = "Locked aid: unavailable".to_string();
        }
    }
}

fn relation_summary(relation: &DiplomaticRelation) -> &'static str {
    match relation.band() {
        RelationshipBand::Hostile => "Open hostility — expect reprisals.",
        RelationshipBand::Unfriendly => "Tense — diplomats exchange harsh words.",
        RelationshipBand::Neutral => "Even — neither warm nor cold.",
        RelationshipBand::Cordial => "Cordial — polite and improving ties.",
        RelationshipBand::Warm => "Warm — strong mutual goodwill.",
        RelationshipBand::Allied => "Allied — steadfast partners in policy.",
    }
}

fn update_action_buttons(
    selection: Res<DiplomacySelection>,
    state: Res<DiplomacyState>,
    ledger: Res<ForeignAidLedger>,
    player: Option<Res<PlayerNation>>,
    mut buttons: Query<(&mut DiplomacyActionButton, &mut Visibility)>,
) {
    let Some(selected) = selection.selected else {
        for (_, mut visibility) in buttons.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let player_instance = player.as_ref().map(|p| p.instance());

    for (mut button, mut visibility) in buttons.iter_mut() {
        button.target = Some(selected);
        let Some(player_inst) = player_instance else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some(relation) = state.relation(player_inst, selected) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let show = match button.action {
            DiplomaticAction::DeclareWar => !relation.treaty.at_war,
            DiplomaticAction::OfferPeace => relation.treaty.at_war,
            DiplomaticAction::Consulate => !relation.treaty.consulate && !relation.treaty.at_war,
            DiplomaticAction::Embassy => {
                relation.treaty.consulate && !relation.treaty.embassy && !relation.treaty.at_war
            }
            DiplomaticAction::Pact => relation.treaty.embassy && !relation.treaty.at_war,
            DiplomaticAction::Alliance => relation.treaty.embassy && !relation.treaty.at_war,
            DiplomaticAction::AidOnce(_) => !relation.treaty.at_war,
            DiplomaticAction::AidLocked(_) => !relation.treaty.at_war,
            DiplomaticAction::CancelAid => ledger.has_recurring(player_inst, selected),
        };

        *visibility = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_pending_offers(
    offers: Res<DiplomaticOffers>,
    player: Option<Res<PlayerNation>>,
    nations: Query<(NationInstance, &Name)>,
    children: Query<&Children>,
    list_query: Query<Entity, With<PendingOfferList>>,
    mut commands: Commands,
) {
    let Some(list_entity) = list_query.iter().next() else {
        return;
    };

    let Some(player) = player.as_ref() else {
        return;
    };

    let player_instance = player.instance();

    if !offers.is_changed() && !player.is_changed() {
        return;
    }

    let mut names: HashMap<NationInstance, String> = HashMap::new();
    for (instance, name) in nations.iter() {
        names.insert(instance, name.0.clone());
    }

    let relevant: Vec<DiplomaticOffer> = offers.iter_for(player_instance).cloned().collect();

    clear_children_recursive(list_entity, &mut commands, &children);

    commands.entity(list_entity).with_children(|list| {
        if relevant.is_empty() {
            list.spawn((
                Text::new("No pending offers."),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.83, 0.9)),
            ));
        } else {
            for offer in relevant {
                let summary = describe_offer(&offer, &names);
                list.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(DETAIL_BG),
                ))
                .with_children(|entry| {
                    entry.spawn((
                        Text::new(summary),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                    ));

                    entry
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            ..default()
                        },))
                        .with_children(|row| {
                            let offer_id = offer.id;
                            row.spawn((
                                Button,
                                OldButton,
                                Node {
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_ACCENT),
                                AccentButton,
                                observe(move |_: On<Activate>,
                                    mut offers: ResMut<DiplomaticOffers>,
                                    mut state: ResMut<DiplomacyState>,
                                    mut ledger: ResMut<ForeignAidLedger>,
                                    nations: Query<(NationInstance, &Name)>,
                                    mut treasuries: Query<&mut Treasury>| {
                                    if let Some(offer) = offers.take(offer_id) {
                                        resolve_offer_response(
                                            offer,
                                            true, // accept
                                            &mut state,
                                            &mut ledger,
                                            &nations,
                                            &mut treasuries,
                                        );
                                    }
                                }),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new("Accept"),
                                    TextFont {
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                ));
                            });

                            row.spawn((
                                Button,
                                OldButton,
                                Node {
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_DANGER),
                                DangerButton,
                                observe(move |_: On<Activate>,
                                    mut offers: ResMut<DiplomaticOffers>,
                                    mut state: ResMut<DiplomacyState>,
                                    mut ledger: ResMut<ForeignAidLedger>,
                                    nations: Query<(NationInstance, &Name)>,
                                    mut treasuries: Query<&mut Treasury>| {
                                    if let Some(offer) = offers.take(offer_id) {
                                        resolve_offer_response(
                                            offer,
                                            false, // decline
                                            &mut state,
                                            &mut ledger,
                                            &nations,
                                            &mut treasuries,
                                        );
                                    }
                                }),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new("Decline"),
                                    TextFont {
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.92, 0.95, 1.0)),
                                ));
                            });
                        });
                });
            }
        }
    });
}

fn clear_children_recursive(entity: Entity, commands: &mut Commands, children: &Query<&Children>) {
    if let Ok(child_list) = children.get(entity) {
        for child in child_list.iter() {
            clear_children_recursive(child, commands, children);
            commands.entity(child).despawn();
        }
    }
}

fn describe_offer(offer: &DiplomaticOffer, names: &HashMap<NationInstance, String>) -> String {
    match &offer.kind {
        DiplomaticOfferKind::OfferPeace => {
            format!("{} requests peace.", format_name(names, offer.from))
        }
        DiplomaticOfferKind::Alliance => {
            format!(
                "{} proposes a mutual alliance.",
                format_name(names, offer.from)
            )
        }
        DiplomaticOfferKind::NonAggressionPact => {
            format!(
                "{} seeks a non-aggression pact.",
                format_name(names, offer.from)
            )
        }
        DiplomaticOfferKind::ForeignAid { amount, locked } => {
            if *locked {
                format!(
                    "{} offers a locked ${} annual grant.",
                    format_name(names, offer.from),
                    amount
                )
            } else {
                format!(
                    "{} offers a one-time aid payment of ${}.",
                    format_name(names, offer.from),
                    amount
                )
            }
        }
        DiplomaticOfferKind::JoinWar { enemy, defensive } => {
            if *defensive {
                format!(
                    "{} invokes your alliance against {}.",
                    format_name(names, offer.from),
                    format_name(names, *enemy)
                )
            } else {
                format!(
                    "{} requests support in their war on {}.",
                    format_name(names, offer.from),
                    format_name(names, *enemy)
                )
            }
        }
    }
}

fn format_name(names: &HashMap<NationInstance, String>, nation: NationInstance) -> String {
    names
        .get(&nation)
        .cloned()
        .unwrap_or_else(|| format!("Nation {:?}", nation.entity()))
}

// Note: hide_diplomacy_screen replaced with generic hide_screen::<DiplomacyScreen>
// See src/ui/generic_systems.rs for the generic implementation
