use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Good {
    Wool,
    Cotton,
    Cloth,
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Good::Wool => write!(f, "Wool"),
            Good::Cotton => write!(f, "Cotton"),
            Good::Cloth => write!(f, "Cloth"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats() {
        assert_eq!(Good::Wool.to_string(), "Wool");
        assert_eq!(Good::Cotton.to_string(), "Cotton");
        assert_eq!(Good::Cloth.to_string(), "Cloth");
    }
}
