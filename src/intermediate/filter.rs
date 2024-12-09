#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Filter method 0. See https://www.w3.org/TR/png-3/#9Filter-types
    /// Currently the only supported filter method
    #[default]
    Zero,
}

impl TryFrom<u8> for Filter {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            _ => Err("Unknown filter method"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    #[default]
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl TryFrom<u8> for FilterKind {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Sub),
            2 => Ok(Self::Up),
            3 => Ok(Self::Average),
            4 => Ok(Self::Paeth),
            _ => Err("Unknown filter type"),
        }
    }
}
