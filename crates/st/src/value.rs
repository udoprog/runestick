use crate::external::External;
use crate::vm::{Ref, StackError, Vm};
use std::any::TypeId;
use std::fmt;

/// Describes what slot error happened.
#[derive(Debug, Clone, Copy)]
pub enum ValueError {
    /// A string slot could not be looked up.
    String(usize),
    /// An array slot could not be looked up.
    Array(usize),
    /// An external could not be looked up.
    External(usize),
    /// A dynamic value could not be looked up.
    Value(usize),
}

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum Value {
    /// An empty unit.
    Unit,
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// Reference to an external type.
    External(Box<dyn External>),
}

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum ValueRef<'a> {
    /// An empty unit.
    Unit,
    /// A string.
    String(Ref<'a, String>),
    /// An array.
    Array(Vec<ValueRef<'a>>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// Reference to an external type.
    External(Ref<'a, dyn External>),
}

/// Managed entries on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Managed {
    /// A string.
    String,
    /// An array.
    Array,
    /// Reference to an external type.
    External,
}

impl fmt::Display for Managed {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::String => write!(fmt, "string"),
            Self::Array => write!(fmt, "array"),
            Self::External => write!(fmt, "external"),
        }
    }
}

/// Compact information on typed slot.
#[derive(Clone, Copy)]
pub struct Slot(usize);

impl Slot {
    const STRING: usize = 0;
    const ARRAY: usize = 1;
    const EXTERNAL: usize = 2;

    /// Slot
    pub fn into_managed(self) -> (Managed, usize) {
        let slot = (self.0 >> 2) as usize;

        match self.0 & 0b11 {
            0 => (Managed::String, slot),
            1 => (Managed::Array, slot),
            _ => (Managed::External, slot),
        }
    }

    /// Construct a string slot.
    pub fn string(slot: usize) -> Self {
        Self(slot << 2 | Self::STRING)
    }

    /// Construct an array slot.
    pub fn array(slot: usize) -> Self {
        Self(slot << 2 | Self::ARRAY)
    }

    /// Construct an external slot.
    pub fn external(slot: usize) -> Self {
        Self(slot << 2 | Self::EXTERNAL)
    }
}

impl fmt::Debug for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (managed, slot) = self.into_managed();
        write!(fmt, "{}({})", managed, slot)
    }
}

macro_rules! decl_managed {
    ($name:ident, $constant:ident) => {
        #[allow(unused)]
        struct $name(());

        impl IntoSlot for $name {
            fn into_slot(value: ValuePtr) -> Result<usize, StackError> {
                let Slot(slot) = match value {
                    ValuePtr::Managed(managed) => managed,
                    _ => return Err(StackError::ExpectedManaged),
                };

                if slot & 0b11 == Slot::$constant {
                    Ok((slot >> 2) as usize)
                } else {
                    Err(StackError::IncompatibleSlot)
                }
            }
        }
    };
}

decl_managed!(ManagedString, STRING);
decl_managed!(ManagedArray, ARRAY);
decl_managed!(ManagedExternal, EXTERNAL);

/// Trait for converting into managed slots.
trait IntoSlot {
    /// Convert thing into a managed slot.
    fn into_slot(value: ValuePtr) -> Result<usize, StackError>;
}

/// An entry on the stack.
#[derive(Debug, Clone, Copy)]
pub enum ValuePtr {
    /// An empty unit.
    Unit,
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A managed reference.
    Managed(Slot),
}

impl ValuePtr {
    /// Convert value into a managed.
    #[inline]
    pub fn into_managed(self) -> Result<(Managed, usize), StackError> {
        if let Self::Managed(slot) = self {
            Ok(slot.into_managed())
        } else {
            Err(StackError::ExpectedManaged)
        }
    }

    /// Try to convert into managed.
    #[inline]
    pub fn try_into_managed(self) -> Option<(Managed, usize)> {
        if let Self::Managed(slot) = self {
            Some(slot.into_managed())
        } else {
            None
        }
    }

    /// Convert value into a managed slot.
    #[inline]
    fn into_slot<T>(self) -> Result<usize, StackError>
    where
        T: IntoSlot,
    {
        T::into_slot(self)
    }

    /// Try to coerce value reference into an external.
    pub fn into_external(self) -> Result<usize, StackError> {
        self.into_slot::<ManagedExternal>()
    }

    /// Try to coerce value reference into an array.
    pub fn into_array(self) -> Result<usize, StackError> {
        self.into_slot::<ManagedArray>()
    }

    /// Try to coerce value reference into an array.
    pub fn into_string(self) -> Result<usize, StackError> {
        self.into_slot::<ManagedString>()
    }

    /// Get the type information for the current value.
    pub fn value_type(&self, vm: &Vm) -> Result<ValueType, StackError> {
        Ok(match *self {
            Self::Unit => ValueType::Unit,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::Managed(slot) => match slot.into_managed() {
                (Managed::String, ..) => ValueType::String,
                (Managed::Array, _) => ValueType::Array,
                (Managed::External, slot) => {
                    let (_, type_hash) = vm.external_type(slot)?;

                    ValueType::External(type_hash)
                }
            },
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, StackError> {
        Ok(match *self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Managed(slot) => match slot.into_managed() {
                (Managed::String, _) => ValueTypeInfo::String,
                (Managed::Array, _) => ValueTypeInfo::Array,
                (Managed::External, slot) => {
                    let (type_name, type_hash) = vm.external_type(slot)?;

                    ValueTypeInfo::External(type_name, type_hash)
                }
            },
        })
    }
}

impl Default for ValuePtr {
    fn default() -> Self {
        Self::Unit
    }
}

/// The type of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array of dynamic values.
    Array,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// Reference to a foreign type.
    External(TypeId),
}

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array.
    Array,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// Reference to a foreign type.
    External(&'static str, TypeId),
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unit => write!(fmt, "()"),
            Self::String => write!(fmt, "String"),
            Self::Array => write!(fmt, "Array"),
            Self::Integer => write!(fmt, "Integer"),
            Self::Float => write!(fmt, "Float"),
            Self::Bool => write!(fmt, "Bool"),
            Self::External(name, _) => write!(fmt, "External({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Slot, ValuePtr, ValueType};

    #[test]
    fn test_slot() {
        assert_eq!(Slot::string(4).into_managed(), (crate::Managed::String, 4));
        assert_eq!(Slot::array(4).into_managed(), (crate::Managed::Array, 4));
        assert_eq!(
            Slot::external(4).into_managed(),
            (crate::Managed::External, 4)
        );
    }

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<ValuePtr>(),
            16,
        };

        assert_eq! {
            std::mem::size_of::<ValueType>(),
            16,
        };
    }
}