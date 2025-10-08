/// Calculate recruitment cap based on province count
pub fn calculate_recruitment_cap(province_count: u32, upgraded: bool) -> u32 {
    if upgraded {
        province_count / 3
    } else {
        province_count / 4
    }
}
