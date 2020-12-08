use crate::internal::commas;
use crate::ValueId;
use std::fmt;

/// The definition of an input to a block.
///
/// These are essentially phi nodes, and makes sure that there's a local
/// variable declaration available.
#[derive(Debug, Clone, Default)]
pub struct Phi {
    /// Dependencies to this input value.
    dependencies: Vec<ValueId>,
}

impl Phi {
    /// Push a dependency onto this phi node.
    pub(crate) fn push(&mut self, value: ValueId) {
        self.dependencies.push(value);
    }
}

impl fmt::Display for Phi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dependencies.is_empty() {
            write!(f, "Φ(?)")?;
        } else {
            write!(f, "Φ({})", commas(&self.dependencies))?;
        }

        Ok(())
    }
}
