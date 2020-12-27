use crate::{ConstId, Dep, Phi, Var};
use std::fmt;

/// A single abstract machine instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Inst {
    /// A numerated input.
    Input(usize),
    /// An instruction to load a constant as a value.
    Const(ConstId),
    /// An instruction directly references a different value.
    Var(Dep),
    /// A phony use node, indicating what assignments flow into this.
    Phi(Phi),
    /// Compute `lhs + rhs`.
    Add(Var, Var),
    /// Compute `lhs - rhs`.
    Sub(Var, Var),
    /// Compute `lhs / rhs`.
    Div(Var, Var),
    /// Compute `lhs * rhs`.
    Mul(Var, Var),
    /// Compare if `lhs < rhs`.
    CmpLt(Var, Var),
    /// Compare if `lhs <= rhs`.
    CmpLte(Var, Var),
    /// Compare if `lhs == rhs`.
    CmpEq(Var, Var),
    /// Compare if `lhs > rhs`.
    CmpGt(Var, Var),
    /// Compare if `lhs >= rhs`.
    CmpGte(Var, Var),
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
            Inst::Input(n) => {
                write!(f, "input {}", n)?;
            }
            Inst::Const(id) => {
                write!(f, "{}", id)?;
            }
            Inst::Var(dep) => {
                write!(f, "{}", dep)?;
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
