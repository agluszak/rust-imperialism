use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

use super::allocation_widgets::{
    AllocationBar, AllocationStepperDisplay, AllocationSummary, AllocationType,
};
use crate::economy::{Allocations, PlayerNation, Stockpile, Treasury};
use crate::messages::{
    AdjustMarketOrder, AdjustProduction, AdjustRecruitment, AdjustTraining, MarketInterest,
};

// ============================================================================
// Input Layer: Unified stepper button handler
// ============================================================================

/// Creates an observer that adjusts allocation when a stepper button is activated
pub fn adjust_allocation_on_click(allocation_type: AllocationType, delta: i32) -> impl Bundle {
    observe(
        move |_activate: On<Activate>,
              player_nation: Option<Res<PlayerNation>>,
              allocations: Query<&Allocations>,
              mut recruit_writer: MessageWriter<AdjustRecruitment>,
              mut train_writer: MessageWriter<AdjustTraining>,
              mut prod_writer: MessageWriter<AdjustProduction>,
              mut market_writer: MessageWriter<AdjustMarketOrder>| {
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
                    let current = alloc.recruitment_count() as u32;
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    recruit_writer.write(AdjustRecruitment {
                        nation: player_instance,
                        requested: new_requested,
                    });
                    info!(
                        "Recruitment: {} -> {} (delta: {})",
                        current, new_requested, delta
                    );
                }

                AllocationType::Training(from_skill) => {
                    let current = alloc.training_count(from_skill) as u32;
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    train_writer.write(AdjustTraining {
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
                    let current = alloc.production_count(building_entity, output_good) as u32;
                    let new_target = (current as i32 + delta).max(0) as u32;
                    prod_writer.write(AdjustProduction {
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
                    // Buy interest is boolean - toggle between 0 and 1
                    let current = if alloc.has_buy_interest(good) { 1 } else { 0 };
                    let new_requested = if current == 0 { 1 } else { 0 };
                    market_writer.write(AdjustMarketOrder {
                        nation: player_instance,
                        good,
                        kind: MarketInterest::Buy,
                        requested: new_requested,
                    });
                    info!(
                        "Market buy interest ({:?}): {} -> {}",
                        good,
                        if current == 1 { "ON" } else { "OFF" },
                        if new_requested == 1 { "ON" } else { "OFF" }
                    );
                }

                AllocationType::MarketSell(good) => {
                    let current = alloc.market_sell_count(good) as u32;
                    let new_requested = (current as i32 + delta).max(0) as u32;
                    market_writer.write(AdjustMarketOrder {
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
            let allocated = match display.allocation_type {
                AllocationType::Recruitment => alloc.recruitment_count(),

                AllocationType::Training(from_skill) => alloc.training_count(from_skill),

                AllocationType::Production(building_entity, output_good) => {
                    alloc.production_count(building_entity, output_good)
                }

                AllocationType::MarketBuy(good) => {
                    if alloc.has_buy_interest(good) {
                        1
                    } else {
                        0
                    }
                }

                AllocationType::MarketSell(good) => alloc.market_sell_count(good),
            };

            // With new system, allocated is always what's been successfully reserved
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
    buildings_query: Query<&crate::economy::production::Buildings>,
    mut bars: Query<(
        &mut Text,
        &mut BackgroundColor,
        &mut BorderColor,
        &AllocationBar,
    )>,
    allocations_changed: Query<Entity, Changed<Allocations>>,
    new_bars: Query<Entity, Added<AllocationBar>>,
) {
    use crate::economy::{BuildingKind, Good};

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

    let Ok(buildings_collection) = buildings_query.get(player_entity) else {
        return;
    };

    let Ok(_treasury) = treasuries.get(player_entity) else {
        return;
    };

    for (mut text, mut bg_color, mut border_color, bar) in bars.iter_mut() {
        // Calculate needed based on allocation type
        // Each allocation represents 1 unit, so needed = count * per-unit-cost
        let (needed, available) = match bar.allocation_type {
            AllocationType::Recruitment => {
                let count = alloc.recruitment_count() as u32;
                let available = stockpile.get_available(bar.good);
                let needed = match bar.good {
                    Good::CannedFood | Good::Clothing | Good::Furniture => count,
                    _ => 0,
                };
                (needed, available)
            }

            AllocationType::Training(from_skill) => {
                let count = alloc.training_count(from_skill) as u32;
                let available = stockpile.get_available(bar.good);
                let needed = match bar.good {
                    Good::Paper => count,
                    _ => 0,
                };
                (needed, available)
            }

            AllocationType::Production(building_entity, output_good) => {
                let available = stockpile.get_available(bar.good);
                let count = alloc.production_count(building_entity, output_good) as u32;
                if count == 0 {
                    (0, available)
                } else {
                    let building_kind = match output_good {
                        Good::Fabric => BuildingKind::TextileMill,
                        Good::Paper | Good::Lumber => BuildingKind::LumberMill,
                        Good::Steel => BuildingKind::SteelMill,
                        Good::CannedFood => BuildingKind::FoodProcessingCenter,
                        Good::Clothing => BuildingKind::ClothingFactory,
                        Good::Furniture => BuildingKind::FurnitureFactory,
                        Good::Hardware | Good::Armaments => BuildingKind::MetalWorks,
                        Good::Fuel => BuildingKind::Refinery,
                        Good::Transport => BuildingKind::Railyard,
                        _ => BuildingKind::TextileMill,
                    };

                    if buildings_collection.get(building_kind).is_some() {
                        let per_unit_cost = match (building_kind, bar.good) {
                            (BuildingKind::TextileMill, Good::Cotton) => 2,
                            (BuildingKind::TextileMill, Good::Wool) => 2,
                            (BuildingKind::LumberMill, Good::Timber) => 2,
                            (BuildingKind::SteelMill, Good::Iron) => 1,
                            (BuildingKind::SteelMill, Good::Coal) => 1,
                            (BuildingKind::FoodProcessingCenter, Good::Grain) => 2,
                            (BuildingKind::FoodProcessingCenter, Good::Fruit) => 1,
                            (BuildingKind::FoodProcessingCenter, Good::Livestock) => 1,
                            (BuildingKind::FoodProcessingCenter, Good::Fish) => 1,
                            (BuildingKind::ClothingFactory, Good::Fabric) => 2,
                            (BuildingKind::FurnitureFactory, Good::Lumber) => 2,
                            (BuildingKind::MetalWorks, Good::Steel) => 2,
                            (BuildingKind::Refinery, Good::Oil) => 2,
                            (BuildingKind::Railyard, Good::Steel) => 1,
                            (BuildingKind::Railyard, Good::Lumber) => 1,
                            _ => 0,
                        };
                        (count * per_unit_cost, available)
                    } else {
                        (0, available)
                    }
                }
            }

            AllocationType::MarketBuy(good) => {
                // Buy interest is just a flag, no resources needed/reserved
                let _has_interest = alloc.has_buy_interest(good);
                (0, 0)
            }

            AllocationType::MarketSell(good) => {
                let count = alloc.market_sell_count(good) as u32;
                let available = stockpile.get_available(good);
                (count, available)
            }
        };

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
                    let allocated = alloc.recruitment_count();
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
                    let allocated = alloc.training_count(from_skill);
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

                AllocationType::Production(building_entity, output_good) => {
                    let allocated = alloc.production_count(building_entity, output_good);
                    if allocated > 0 {
                        format!("-> Will produce {} {:?} next turn", allocated, output_good)
                    } else {
                        "-> No production planned".to_string()
                    }
                }

                AllocationType::MarketBuy(good) => {
                    if alloc.has_buy_interest(good) {
                        format!("-> Interested in buying {}", good)
                    } else {
                        format!("-> No buy interest for {}", good)
                    }
                }

                AllocationType::MarketSell(good) => {
                    let allocated = alloc.market_sell_count(good);
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
