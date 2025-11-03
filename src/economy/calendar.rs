use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl core::fmt::Display for Season {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Season::Spring => write!(f, "Spring"),
            Season::Summer => write!(f, "Summer"),
            Season::Autumn => write!(f, "Autumn"),
            Season::Winter => write!(f, "Winter"),
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calendar {
    pub season: Season,
    pub year: u16,
}

impl Default for Calendar {
    fn default() -> Self {
        Calendar {
            season: Season::Spring,
            year: 1815,
        }
    }
}

impl Calendar {
    pub fn display(&self) -> String {
        format!("{}, {}", self.season, self.year)
    }
}

#[cfg(test)]
mod tests {
    use crate::economy::*;

    #[test]
    fn calendar_display() {
        let c = Calendar::default();
        assert_eq!(c.display(), "Spring, 1815");
    }
}
