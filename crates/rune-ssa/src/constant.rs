use std::fmt;

/// A constant value.
pub enum Constant {
    /// The unit constant (always has constant id = 0).
    Unit,
    /// A boolean constant.
    Bool(bool),
    /// A character constant.
    Char(char),
    /// A byte constant.
    Byte(u8),
    /// An integer constant.
    Integer(i64),
    /// A float constant.
    Float(f64),
    /// A string constant.
    String(Box<str>),
    /// A byte constant.
    Bytes(Box<[u8]>),
}

impl fmt::Debug for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Unit => {
                write!(f, "()")?;
            }
            Constant::Bool(b) => {
                write!(f, "{}", b)?;
            }
            Constant::Char(c) => {
                write!(f, "{:?}", c)?;
            }
            Constant::Byte(b) => {
                write!(f, "0x{:02x}", b)?;
            }
            Constant::Integer(n) => {
                write!(f, "{}", n)?;
            }
            Constant::Float(n) => {
                write!(f, "{}", n)?;
            }
            Constant::String(s) => {
                write!(f, "{:?}", s)?;
            }
            Constant::Bytes(b) => {
                write!(f, "{:?}", b)?;
            }
        }

        Ok(())
    }
}
