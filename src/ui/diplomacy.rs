use std::collections::HashMap;

use bevy::prelude::*;

use super::button_style::{
    AccentButton, DangerButton, NORMAL_ACCENT, NORMAL_BUTTON, NORMAL_DANGER,
};
use super::generic_systems::hide_screen;
use crate::diplomacy::{
    DiplomacySelection, DiplomacyState, DiplomaticOffer, DiplomaticOfferKind, DiplomaticOffers,
    DiplomaticOrder, DiplomaticOrderKind, ForeignAidLedger, OfferId, resolve_offer_response,
};
use crate::economy::{Name, NationId, PlayerNation, Treasury};
use crate::ui::logging::TerminalLogEvent;
use crate::ui::mode::{GameMode, MapModeButton};

const PANEL_BG: Color = Color::srgba(0.08, 0.09, 0.12, 0.92);
const LIST_BG: Color = Color::srgba(0.11, 0.12, 0.15, 0.85);
const DETAIL_BG: Color = Color::srgba(0.14, 0.15, 0.18, 0.75);

#[derive(Component)]
pub struct DiplomacyScreen;

#[derive(Component)]
struct DiplomacyNationButton {
    nation: NationId,
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
struct DiplomacyActionButton {
    action: DiplomaticAction,
    target: NationId,
}

#[derive(Component)]
struct PendingOffersContainer;

#[derive(Component)]
struct OfferResponseButton {
    offer: OfferId,
    accept: bool,
}

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
                handle_nation_selection,
                update_action_buttons,
                handle_action_buttons,
                update_pending_offers,
                handle_offer_response_buttons,
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
    nations: Query<(Entity, &NationId, &Name)>,
    player: Option<Res<PlayerNation>>,
    mut selection: ResMut<DiplomacySelection>,
) {
    if let Ok(mut visibility) = screen_visibility.single_mut() {
        *visibility = Visibility::Visible;
        return;
    }

    let player_entity = player.as_ref().map(|p| *p.0);

    let mut foreign_nations: Vec<(NationId, String)> = nations
        .iter()
        .filter_map(|(entity, id, name)| {
            if Some(entity) == player_entity {
                None
            } else {
                Some((*id, name.0.clone()))
            }
        })
        .collect();
    foreign_nations.sort_by(|a, b| a.1.cmp(&b.1));

    if selection
        .selected
        .map(|sel| foreign_nations.iter().any(|(id, _)| *id == sel))
        .unwrap_or(false)
    {
        // Keep existing selection
    } else {
        selection.selected = foreign_nations.first().map(|(id, _)| *id);
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

                        for (nation, name) in &foreign_nations {
                            list.spawn((
                                Button,
                                Node {
                                    padding: UiRect::all(Val::Px(8.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_BUTTON),
                                DiplomacyNationButton { nation: *nation },
                            ))
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_DANGER),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::DeclareWar,
                                        target: NationId(0),
                                    },
                                    DangerButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::OfferPeace,
                                        target: NationId(0),
                                    },
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Consulate,
                                        target: NationId(0),
                                    },
                                    AccentButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Embassy,
                                        target: NationId(0),
                                    },
                                    AccentButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Pact,
                                        target: NationId(0),
                                    },
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::Alliance,
                                        target: NationId(0),
                                    },
                                    AccentButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::AidOnce(500),
                                        target: NationId(0),
                                    },
                                    AccentButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_ACCENT),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::AidLocked(500),
                                        target: NationId(0),
                                    },
                                    AccentButton,
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
                                    Node {
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_BUTTON),
                                    DiplomacyActionButton {
                                        action: DiplomaticAction::CancelAid,
                                        target: NationId(0),
                                    },
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
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                MapModeButton,
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
    nation_ids: Query<(Entity, &NationId)>,
) {
    let player_entity = player.map(|p| *p.0);
    let mut available: Vec<NationId> = Vec::new();
    for (entity, nation) in nation_ids.iter() {
        if Some(entity) == player_entity {
            continue;
        }
        available.push(*nation);
    }

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
    nation_ids: Query<&NationId>,
    names: Query<(&NationId, &Name)>,
    mut buttons: Query<
        (&DiplomacyNationButton, &mut Text, &mut BackgroundColor),
        (
            Without<SelectedNationNameText>,
            Without<SelectedRelationText>,
            Without<SelectedTreatyText>,
            Without<SelectedAidText>,
        ),
    >,
) {
    let player_id = player.and_then(|p| nation_ids.get(*p.0).ok()).copied();

    for (button, mut text, mut color) in buttons.iter_mut() {
        let label = names
            .iter()
            .find(|(id, _)| **id == button.nation)
            .map(|(_, name)| name.0.clone())
            .unwrap_or_else(|| format!("Nation {}", button.nation.0));

        let mut relation_line = label.clone();
        if let Some(player_id) = player_id
            && let Some(relation) = state.relation(player_id, button.nation)
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

fn handle_nation_selection(
    mut selection: ResMut<DiplomacySelection>,
    mut buttons: Query<(&Interaction, &DiplomacyNationButton), Changed<Interaction>>,
) {
    for (interaction, button) in buttons.iter_mut() {
        if *interaction == Interaction::Pressed {
            selection.selected = Some(button.nation);
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_detail_panel(
    mode: Res<State<GameMode>>,
    selection: Res<DiplomacySelection>,
    state: Res<DiplomacyState>,
    ledger: Res<ForeignAidLedger>,
    player: Option<Res<PlayerNation>>,
    nation_ids: Query<&NationId>,
    names: Query<(&NationId, &Name)>,
    mut text_queries: ParamSet<(
        Query<&mut Text, (With<SelectedNationNameText>, Without<DiplomacyNationButton>)>,
        Query<&mut Text, (With<SelectedRelationText>, Without<DiplomacyNationButton>)>,
        Query<&mut Text, (With<SelectedTreatyText>, Without<DiplomacyNationButton>)>,
        Query<&mut Text, (With<SelectedAidText>, Without<DiplomacyNationButton>)>,
    )>,
) {
    // Manual state check since we can't use run_if with ParamSet
    if mode.get() != &GameMode::Diplomacy {
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
            text.0 = "Treaties: none".to_string();
        }
        if let Ok(mut text) = text_queries.p3().single_mut() {
            text.0 = "Locked aid: none".to_string();
        }
        return;
    };

    let player_id = player.and_then(|p| nation_ids.get(*p.0).ok()).copied();

    let selected_name = names
        .iter()
        .find(|(id, _)| **id == selected)
        .map(|(_, name)| name.0.clone())
        .unwrap_or_else(|| format!("Nation {}", selected.0));

    if let Ok(mut text) = text_queries.p0().single_mut() {
        text.0 = selected_name.clone();
    }

    let relation = player_id.and_then(|pid| state.relation(pid, selected));
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

    if let Ok(mut text) = text_queries.p3().single_mut() {
        if let Some(player_id) = player_id {
            if let Some(grant) = ledger
                .all()
                .iter()
                .find(|g| g.from == player_id && g.to == selected)
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

fn update_action_buttons(
    selection: Res<DiplomacySelection>,
    state: Res<DiplomacyState>,
    ledger: Res<ForeignAidLedger>,
    player: Option<Res<PlayerNation>>,
    nation_ids: Query<&NationId>,
    mut buttons: Query<(&mut DiplomacyActionButton, &mut Visibility)>,
) {
    let Some(selected) = selection.selected else {
        for (_, mut visibility) in buttons.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let player_id = if let Some(player) = player {
        nation_ids.get(*player.0).ok().copied()
    } else {
        None
    };

    for (mut button, mut visibility) in buttons.iter_mut() {
        button.target = selected;
        let Some(player_id) = player_id else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some(relation) = state.relation(player_id, selected) else {
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
            DiplomaticAction::CancelAid => ledger.has_recurring(player_id, selected),
        };

        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn handle_action_buttons(
    selection: Res<DiplomacySelection>,
    player: Option<Res<PlayerNation>>,
    nation_ids: Query<&NationId>,
    mut buttons: Query<(&Interaction, &DiplomacyActionButton), Changed<Interaction>>,
    mut orders: MessageWriter<DiplomaticOrder>,
) {
    let Some(selected) = selection.selected else {
        return;
    };

    let player_id = match player.and_then(|p| nation_ids.get(*p.0).ok()).copied() {
        Some(id) => id,
        None => return,
    };

    for (interaction, button) in buttons.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if button.target != selected {
            continue;
        }

        let order = match button.action {
            DiplomaticAction::DeclareWar => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::DeclareWar,
            },
            DiplomaticAction::OfferPeace => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::OfferPeace,
            },
            DiplomaticAction::Consulate => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::EstablishConsulate,
            },
            DiplomaticAction::Embassy => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::OpenEmbassy,
            },
            DiplomaticAction::Pact => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::SignNonAggressionPact,
            },
            DiplomaticAction::Alliance => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::FormAlliance,
            },
            DiplomaticAction::AidOnce(amount) => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::SendAid {
                    amount,
                    locked: false,
                },
            },
            DiplomaticAction::AidLocked(amount) => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::SendAid {
                    amount,
                    locked: true,
                },
            },
            DiplomaticAction::CancelAid => DiplomaticOrder {
                actor: player_id,
                target: selected,
                kind: DiplomaticOrderKind::CancelAid,
            },
        };

        orders.write(order);
    }
}

fn update_pending_offers(
    offers: Res<DiplomaticOffers>,
    player: Option<Res<PlayerNation>>,
    nation_ids: Query<&NationId>,
    nations: Query<(Entity, &NationId, &Name)>,
    children: Query<&Children>,
    list_query: Query<Entity, With<PendingOfferList>>,
    mut commands: Commands,
) {
    let Some(list_entity) = list_query.iter().next() else {
        return;
    };

    let Some(player) = player else {
        return;
    };

    let Ok(player_id) = nation_ids.get(*player.0) else {
        return;
    };

    if !offers.is_changed() && !player.is_changed() {
        return;
    }

    let mut names: HashMap<NationId, String> = HashMap::new();
    for (_, id, name) in nations.iter() {
        names.insert(*id, name.0.clone());
    }

    let relevant: Vec<DiplomaticOffer> = offers.iter_for(*player_id).cloned().collect();

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
                            row.spawn((
                                Button,
                                Node {
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_ACCENT),
                                OfferResponseButton {
                                    offer: offer.id,
                                    accept: true,
                                },
                                AccentButton,
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
                                Node {
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(NORMAL_DANGER),
                                OfferResponseButton {
                                    offer: offer.id,
                                    accept: false,
                                },
                                DangerButton,
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

fn handle_offer_response_buttons(
    mut interactions: Query<
        (&Interaction, &OfferResponseButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut offers: ResMut<DiplomaticOffers>,
    mut state: ResMut<DiplomacyState>,
    mut ledger: ResMut<ForeignAidLedger>,
    nations: Query<(Entity, &NationId, &Name)>,
    mut treasuries: Query<&mut Treasury>,
    mut log: MessageWriter<TerminalLogEvent>,
) {
    for (interaction, button) in interactions.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(offer) = offers.take(button.offer) else {
            continue;
        };

        resolve_offer_response(
            offer,
            button.accept,
            &mut state,
            &mut ledger,
            &nations,
            &mut treasuries,
            &mut log,
        );
    }
}

fn clear_children_recursive(entity: Entity, commands: &mut Commands, children: &Query<&Children>) {
    if let Ok(child_list) = children.get(entity) {
        for child in child_list.iter() {
            clear_children_recursive(child, commands, children);
            commands.entity(child).despawn();
        }
    }
}

fn describe_offer(offer: &DiplomaticOffer, names: &HashMap<NationId, String>) -> String {
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
    }
}

fn format_name(names: &HashMap<NationId, String>, nation: NationId) -> String {
    names
        .get(&nation)
        .cloned()
        .unwrap_or_else(|| format!("Nation {}", nation.0))
}

// Note: hide_diplomacy_screen replaced with generic hide_screen::<DiplomacyScreen>
// See src/ui/generic_systems.rs for the generic implementation
