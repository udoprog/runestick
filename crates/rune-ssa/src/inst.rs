use crate::{BlockJump, ConstId, Phi, ValueId};
use std::fmt;

/// A single abstract machine instruction.
pub enum Inst {
    /// An instruction to load a constant as a value.
    Const(ConstId),
    /// An instruction directly references a different value.
    Value(ValueId),
    /// A phony use node, indicating what assignments flow into this.
    Phi(Phi),
    /// Compute `lhs + rhs`.
    Add(ValueId, ValueId),
    /// Compute `lhs - rhs`.
    Sub(ValueId, ValueId),
    /// Compute `lhs / rhs`.
    Div(ValueId, ValueId),
    /// Compute `lhs * rhs`.
    Mul(ValueId, ValueId),
    /// Conditionally jump to the given block if the given condition is true.
    JumpIf(ValueId, BlockJump),
    /// Unconditionally jump to the given block if the given condition is true.
    Jump(BlockJump),
    /// Return from the current procedure with the given value.
    Return(ValueId),
    /// Compare if `lhs < rhs`.
    CmpLt(ValueId, ValueId),
    /// Compare if `lhs <= rhs`.
    CmpLte(ValueId, ValueId),
    /// Compare if `lhs == rhs`.
    CmpEq(ValueId, ValueId),
    /// Compare if `lhs > rhs`.
    CmpGt(ValueId, ValueId),
    /// Compare if `lhs >= rhs`.
    CmpGte(ValueId, ValueId),
}

impl Inst {
    /// Dump diagnostical information on an instruction.
    pub fn dump(&self) -> InstDump<'_> {
        InstDump(self)
    }
}

pub struct InstDump<'a>(&'a Inst);

impl fmt::Display for InstDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Inst::Const(id) => {
                write!(f, "{}", id)?;
            }
            Inst::Value(v) => {
                write!(f, "{}", v)?;
            }
            Inst::Phi(phi) => {
                write!(f, "{}", phi)?;
            }
            Inst::Add(lhs, rhs) => {
                write!(f, "add {}, {}", lhs, rhs)?;
            }
            Inst::Sub(lhs, rhs) => {
                write!(f, "sub {}, {}", lhs, rhs)?;
            }
            Inst::Div(lhs, rhs) => {
                write!(f, "div {}, {}", lhs, rhs)?;
            }
            Inst::Mul(lhs, rhs) => {
                write!(f, "mul {}, {}", lhs, rhs)?;
            }
            Inst::JumpIf(cond, block) => {
                write!(f, "jump-if {}, {}", cond, block)?;
            }
            Inst::Jump(block) => {
                write!(f, "jump {}", block)?;
            }
            Inst::Return(value) => {
                write!(f, "return {}", value)?;
            }
            Inst::CmpLt(lhs, rhs) => {
                write!(f, "lt {}, {}", lhs, rhs)?;
            }
            Inst::CmpLte(lhs, rhs) => {
                write!(f, "lte {}, {}", lhs, rhs)?;
            }
            Inst::CmpEq(lhs, rhs) => {
                write!(f, "eq {}, {}", lhs, rhs)?;
            }
            Inst::CmpGt(lhs, rhs) => {
                write!(f, "gt {}, {}", lhs, rhs)?;
            }
            Inst::CmpGte(lhs, rhs) => {
                write!(f, "gte {}, {}", lhs, rhs)?;
            }
        }

        Ok(())
    }
}
