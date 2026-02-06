use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

use crate::economy::{
    Allocations, Good, PlayerNation, Stockpile, Treasury,
    buildings::{Buildings, building_for_output, input_requirement_per_unit},
};
use crate::messages::{
    AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
};
use crate::ui::city::allocation_widgets::{
    AllocationBar, AllocationStepperDisplay, AllocationSummary, AllocationType,
};

fn allocation_value(alloc: &Allocations, allocation_type: AllocationType) -> u32 {
    match allocation_type {
        AllocationType::Recruitment => alloc.recruitment_count() as u32,
        AllocationType::Training(skill) => alloc.training_count(skill) as u32,
        AllocationType::Production(entity, good) => alloc.production_count(entity, good) as u32,
        AllocationType::MarketBuy(good) => {
            // Buy interest is boolean: 1 if interested, 0 if not
            if alloc.has_buy_interest(good) { 1 } else { 0 }
        }
        AllocationType::MarketSell(good) => alloc.market_sell_count(good) as u32,
    }
}

fn recruitment_cost_per_unit(good: Good) -> u32 {
    matches!(good, Good::CannedFood | Good::Clothing | Good::Furniture) as u32
}

fn training_cost_per_unit(good: Good) -> u32 {
    (good == Good::Paper) as u32
}

fn allocation_requirement(
    alloc: &Allocations,
    stockpile: &Stockpile,
    buildings: Option<&Buildings>,
    bar: &AllocationBar,
) -> (u32, u32) {
    match bar.allocation_type {
        AllocationType::Recruitment => {
            let count = allocation_value(alloc, bar.allocation_type);
            let needed = count * recruitment_cost_per_unit(bar.good);
            (needed, stockpile.get_available(bar.good))
        }
        AllocationType::Training(_) => {
            let count = allocation_value(alloc, bar.allocation_type);
            let needed = count * training_cost_per_unit(bar.good);
            (needed, stockpile.get_available(bar.good))
        }
        AllocationType::Production(_, output_good) => {
            let available = stockpile.get_available(bar.good);
            let count = allocation_value(alloc, bar.allocation_type);
            if count == 0 {
                return (0, available);
            }

            if let (Some(buildings), Some(kind)) = (buildings, building_for_output(output_good))
                && buildings.get(kind).is_some()
                && let Some(per_unit) = input_requirement_per_unit(kind, output_good, bar.good)
            {
                return (count * per_unit, available);
            }

            (0, available)
        }
        AllocationType::MarketBuy(_) => (0, 0),
        AllocationType::MarketSell(good) => {
            let count = allocation_value(alloc, bar.allocation_type);
            (count, stockpile.get_available(good))
        }
    }
}

// ============================================================================
// Input Layer: Unified stepper button handler
// ============================================================================

/// Creates an observer that adjusts allocation when a stepper button is activated
pub fn adjust_allocation_on_click(allocation_type: AllocationType, delta: i32) -> impl Bundle {
    observe(
        move |_activate: On<Activate>,
              mut commands: Commands,
              player_nation: Option<Res<PlayerNation>>,
              allocations: Query<&Allocations>| {
            let Some(player) = player_nation else {
                return;
            };

            let player_entity = player.entity();
            let player_instance = player.instance();

            let Ok(alloc) = allocations.get(player_entity) else {
                return;
            };

            match allocation_type {
                AllocationType::Recruitment => {
                    let current = allocation_value(alloc, allocation_type);
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    commands.trigger(AdjustRecruitment {
                        nation: player_instance,
                        requested: new_requested,
                    });
                    info!(
                        "Recruitment: {} -> {} (delta: {})",
                        current, new_requested, delta
                    );
                }

                AllocationType::Training(from_skill) => {
                    let current = allocation_value(alloc, allocation_type);
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    commands.trigger(AdjustTraining {
                        nation: player_instance,
                        from_skill,
                        requested: new_requested,
                    });
                    info!(
                        "Training ({:?}): {} -> {} (delta: {})",
                        from_skill, current, new_requested, delta
                    );
                }

                AllocationType::Production(building_entity, output_good) => {
                    let current = allocation_value(alloc, allocation_type);
                    let new_target = (current as i32 + delta).max(0) as u32;
                    commands.trigger(AdjustProduction {
                        nation: player_instance,
                        building: building_entity,
                        output_good,
                        target_output: new_target,
                    });
                    info!(
                        "Production ({:?}): {} -> {} (delta: {})",
                        output_good, current, new_target, delta
                    );
                }

                AllocationType::MarketBuy(good) => {
                    let current = allocation_value(alloc, allocation_type);
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    commands.trigger(AdjustMarketOrder {
                        nation: player_instance,
                        good,
                        kind: MarketInterest::Buy,
                        requested: new_requested,
                    });
                    info!(
                        "Market buy ({:?}): {} -> {} (delta: {})",
                        good, current, new_requested, delta
                    );
                }

                AllocationType::MarketSell(good) => {
                    let current = allocation_value(alloc, allocation_type);
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    commands.trigger(AdjustMarketOrder {
                        nation: player_instance,
                        good,
                        kind: MarketInterest::Sell,
                        requested: new_requested,
                    });
                    info!(
                        "Market sell ({:?}): {} -> {} (delta: {})",
                        good, current, new_requested, delta
                    );
                }
            }
        },
    )
}

// ============================================================================
// Rendering Layer: Unified display updates
// ============================================================================

/// Update ALL stepper displays (recruitment, training, production)
pub fn update_all_stepper_displays(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    mut displays: Query<(&mut Text, &AllocationStepperDisplay)>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    new_displays: Query<Entity, Added<AllocationStepperDisplay>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    // Only run if allocations changed OR new displays were added
    if allocations_changed.is_empty() && new_displays.is_empty() {
        return;
    }

    if let Ok(alloc) = allocations.get(player.entity()) {
        for (mut text, display) in displays.iter_mut() {
            let allocated = allocation_value(alloc, display.allocation_type);
            text.0 = format!("{}", allocated);
        }
    }
}

/// Update ALL allocation bars (recruitment, training, production)
pub fn update_all_allocation_bars(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    stockpiles: Query<&Stockpile>,
    treasuries: Query<&Treasury>,
    buildings_query: Query<&crate::economy::buildings::Buildings>,
    mut bars: Query<(
        &mut Text,
        &mut BackgroundColor,
        &mut BorderColor,
        &AllocationBar,
    )>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    new_bars: Query<Entity, Added<AllocationBar>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    // Only run if allocations changed OR new bars were added
    if allocations_changed.is_empty() && new_bars.is_empty() {
        return;
    }

    let player_entity = player.entity();

    let Ok(alloc) = allocations.get(player_entity) else {
        return;
    };

    let Ok(stockpile) = stockpiles.get(player_entity) else {
        return;
    };

    let buildings_collection = buildings_query.get(player_entity).ok();

    let Ok(_treasury) = treasuries.get(player_entity) else {
        return;
    };

    for (mut text, mut bg_color, mut border_color, bar) in bars.iter_mut() {
        let (needed, available) =
            allocation_requirement(alloc, stockpile, buildings_collection, bar);

        // Update text
        text.0 = format!("{}: {} / {}", bar.label, needed, available);

        // Color based on constraints
        let (bar_color, border_col) = if needed == 0 {
            // No allocation
            (
                Color::srgba(0.3, 0.3, 0.3, 0.8),
                Color::srgba(0.4, 0.4, 0.4, 0.8),
            )
        } else if needed <= available {
            // Can satisfy
            (
                Color::srgba(0.3, 0.7, 0.3, 0.9),
                Color::srgba(0.4, 0.8, 0.4, 1.0),
            )
        } else {
            // Insufficient
            (
                Color::srgba(0.8, 0.3, 0.3, 0.9),
                Color::srgba(0.9, 0.4, 0.4, 1.0),
            )
        };

        *bg_color = BackgroundColor(bar_color);
        *border_color = BorderColor::all(border_col);
    }
}

/// Update ALL allocation summaries
pub fn update_all_allocation_summaries(
    player_nation: Option<Res<PlayerNation>>,
    allocations: Query<&Allocations>,
    mut summaries: Query<(&mut Text, &AllocationSummary)>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    new_summaries: Query<Entity, Added<AllocationSummary>>,
) {
    let Some(player) = player_nation else {
        return;
    };

    // Only run if allocations changed OR new summaries were added
    if allocations_changed.is_empty() && new_summaries.is_empty() {
        return;
    }

    if let Ok(alloc) = allocations.get(player.entity()) {
        for (mut text, summary) in summaries.iter_mut() {
            text.0 = match summary.allocation_type {
                AllocationType::Recruitment => {
                    let allocated = allocation_value(alloc, summary.allocation_type);
                    if allocated > 0 {
                        format!(
                            "-> Will recruit {} worker{} next turn",
                            allocated,
                            if allocated == 1 { "" } else { "s" }
                        )
                    } else {
                        "-> No workers will be recruited".to_string()
                    }
                }

                AllocationType::Training(from_skill) => {
                    let allocated = allocation_value(alloc, summary.allocation_type);
                    if allocated > 0 {
                        let to_skill = from_skill.next_level();
                        format!(
                            "-> Will train {} worker{} from {:?} to {:?} next turn",
                            allocated,
                            if allocated == 1 { "" } else { "s" },
                            from_skill,
                            to_skill
                        )
                    } else {
                        "-> No workers will be trained".to_string()
                    }
                }

                AllocationType::Production(_, output_good) => {
                    let allocated = allocation_value(alloc, summary.allocation_type);
                    if allocated > 0 {
                        format!("-> Will produce {} {:?} next turn", allocated, output_good)
                    } else {
                        "-> No production planned".to_string()
                    }
                }

                AllocationType::MarketBuy(good) => {
                    let allocated = allocation_value(alloc, summary.allocation_type);
                    if allocated > 0 {
                        let good_name = good.to_string();
                        format!("-> Will bid for {} {} next turn", allocated, good_name)
                    } else {
                        format!("-> No buy orders for {}", good)
                    }
                }

                AllocationType::MarketSell(good) => {
                    let allocated = allocation_value(alloc, summary.allocation_type);
                    if allocated > 0 {
                        format!("-> Will offer {} {} for sale", allocated, good)
                    } else {
                        format!("-> No sell offers for {}", good)
                    }
                }
            };
        }
    }
}
