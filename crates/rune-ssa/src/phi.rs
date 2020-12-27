use crate::internal::commas;
use crate::{BlockId, Var};
use std::collections::BTreeSet;
use std::fmt;

/// The definition of an input to a block.
///
/// These are essentially phi nodes, and makes sure that there's a local
/// variable declaration available.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Phi {
    /// Dependencies to this input value.
    dependencies: BTreeSet<Dep>,
}

impl Phi {
    /// Push a dependency onto this phi node.
    pub(crate) fn insert(&mut self, dep: Dep) {
        self.dependencies.insert(dep);
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

/// A single variable dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dep {
    /// The block the variable is defined in.
    pub block: BlockId,
    /// The variable being depended on.
    pub var: Var,
}

impl fmt::Display for Dep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.block, self.var)
    }
}
