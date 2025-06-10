/// Wrapper for values read from and written to a device.
#[derive(Debug, PartialEq, Clone)]
pub enum Variant {
    Huge(i32),
    Index(u8),
    Integer(i16),
    Logic(bool),
    Real(f32),
    String(String),
}

impl Variant {
    pub fn huge(&self) -> Option<i32> {
        match self {
            Variant::Huge(value) => Some(*value),
            _ => None,
        }
    }

    pub fn index(&self) -> Option<u8> {
        match self {
            Variant::Index(value) => Some(*value),
            _ => None,
        }
    }

    pub fn integer(&self) -> Option<i16> {
        match self {
            Variant::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn logic(&self) -> Option<bool> {
        match self {
            Variant::Logic(value) => Some(*value),
            _ => None,
        }
    }

    pub fn real(&self) -> Option<f32> {
        match self {
            Variant::Real(value) => Some(*value),
            _ => None,
        }
    }

    pub fn string(&self) -> Option<&str> {
        match self {
            Variant::String(value) => Some(value.as_str()),
            _ => None,
        }
    }
}

impl From<i32> for Variant {
    fn from(value: i32) -> Self {
        Variant::Huge(value)
    }
}

impl From<u8> for Variant {
    fn from(value: u8) -> Self {
        Variant::Index(value)
    }
}

impl From<i16> for Variant {
    fn from(value: i16) -> Self {
        Variant::Integer(value)
    }
}

impl From<bool> for Variant {
    fn from(value: bool) -> Self {
        Variant::Logic(value)
    }
}

impl From<f32> for Variant {
    fn from(value: f32) -> Self {
        Variant::Real(value)
    }
}

impl From<String> for Variant {
    fn from(value: String) -> Self {
        Variant::String(value)
    }
}
