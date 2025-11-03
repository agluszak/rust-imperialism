use crate::economy::workforce::Workforce;
use bevy::prelude::*;

/// Calculate recruitment cap based on province count
pub fn calculate_recruitment_cap(province_count: u32, upgraded: bool) -> u32 {
    if upgraded {
        province_count / 3
    } else {
        province_count / 4
    }
}

/// Update labor pools to match current workforce state
/// This should run at the start of each turn to sync labor_pool.total with actual workers
pub fn update_labor_pools(mut workforces: Query<&mut Workforce>) {
    for mut workforce in workforces.iter_mut() {
        workforce.update_labor_pool();
    }
}
