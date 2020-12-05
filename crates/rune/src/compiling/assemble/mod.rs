mod block;
mod builtin_format;
mod builtin_template;
mod const_value;
mod expr;
mod expr_assign;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_continue;
mod expr_field_access;
mod expr_for;
mod expr_if;
mod expr_index;
mod expr_let;
mod expr_loop;
mod expr_match;
mod expr_object;
mod expr_path;
mod expr_range;
mod expr_return;
mod expr_select;
mod expr_try;
mod expr_tuple;
mod expr_unary;
mod expr_vec;
mod expr_while;
mod expr_yield;
mod item_fn;
mod lit;
mod lit_bool;
mod lit_byte;
mod lit_byte_str;
mod lit_char;
mod lit_number;
mod lit_str;
mod local;
mod prelude;

use crate::compiling::{CompileError, CompileResult, Compiler, Needs, VarId, VarOffset};
use runestick::{CompileMetaCapture, Inst, InstAddress, Span};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Value {
    span: Span,
    kind: ValueKind,
}

impl Value {
    /// Construct a value that is not produced at all.
    pub(crate) fn empty(span: Span) -> Self {
        Self {
            span,
            kind: ValueKind::Empty,
        }
    }

    /// Construct a value that is not reachable.
    pub(crate) fn unreachable(span: Span) -> Self {
        Self {
            span,
            kind: ValueKind::Unreachable,
        }
    }

    /// Declare that the assembly resulted in a value in a offset location.
    pub(crate) fn var(span: Span, id: VarId) -> Self {
        Self {
            span,
            kind: ValueKind::Var(id),
        }
    }

    /// Helper to construct a new unnamed value.
    pub(crate) fn unnamed(span: Span, c: &mut Compiler<'_>) -> Self {
        let id = c.scopes.unnamed(span);

        Self {
            span,
            kind: ValueKind::Var(id),
        }
    }

    /// Test if value is empty.
    pub(crate) fn is_present(&self) -> bool {
        matches!(&self.kind, ValueKind::Var(..))
    }

    /// Get the offset of the value.
    pub(crate) fn offset(self, c: &mut Compiler<'_>) -> CompileResult<usize> {
        let id = self.into_var()?;
        Ok(c.scopes.var(self.span, id)?.offset)
    }

    /// Ignore the produced value.
    pub(crate) fn ignore(self, c: &mut Compiler) -> CompileResult<()> {
        match self.kind {
            ValueKind::Unreachable => (),
            ValueKind::Empty => (),
            ValueKind::Var(id) => match c.scopes.offset_of(self.span, id)? {
                VarOffset::Offset(..) => (),
                VarOffset::Top => {
                    c.scopes.stack_pop(self.span)?;
                    c.asm.push(Inst::Pop, self.span);
                }
            },
        }

        Ok(())
    }

    /// Make sure a value is on the top of the stack, with the intent of
    /// immediately consuming it.
    pub(crate) fn pop(self, c: &mut Compiler) -> CompileResult<()> {
        let id = self.into_var()?;

        match c.scopes.offset_of(self.span, id)? {
            VarOffset::Offset(offset) => {
                c.asm.push(Inst::Copy { offset }, self.span);
            }
            VarOffset::Top => {
                let (_, popped) = c.scopes.stack_pop(self.span)?;
                debug_assert!(popped == id);
            }
        }

        Ok(())
    }

    /// Make sure a value is on the top of the stack, with the intent of
    /// consuming it immediately after.
    pub(crate) fn copy(self, c: &mut Compiler) -> CompileResult<()> {
        let id = self.into_var()?;

        match c.scopes.offset_of(self.span, id)? {
            VarOffset::Offset(offset) => {
                c.asm.push(Inst::Copy { offset }, self.span);
            }
            VarOffset::Top => {
                c.asm.push(Inst::Dup, self.span);
            }
        }

        Ok(())
    }

    /// Assemble a non-destructive stack address.
    pub(crate) fn address(self, c: &mut Compiler) -> CompileResult<InstAddress> {
        let id = self.into_var()?;

        let address = match c.scopes.offset_of(self.span, id)? {
            VarOffset::Offset(offset) => InstAddress::Offset(offset),
            VarOffset::Top => InstAddress::Last,
        };

        Ok(address)
    }

    /// Assemble into an address  with the intent of immediately consuming the
    /// stack value if its at the top.
    pub(crate) fn consuming_address(self, c: &mut Compiler) -> CompileResult<InstAddress> {
        let id = self.into_var()?;
        let var = *c.scopes.var(self.span, id)?;

        let address = match c.scopes.offset_of(self.span, id)? {
            VarOffset::Offset(offset) => InstAddress::Offset(offset),
            VarOffset::Top => {
                // Don't consume declared values. These are values which have
                // been given a name somewhere.
                if var.declared {
                    InstAddress::Last
                } else {
                    c.scopes.stack_pop(self.span)?;
                    InstAddress::Top
                }
            }
        };

        Ok(address)
    }

    /// Declare the current value as a variable with the given name.
    pub(crate) fn decl(self, c: &mut Compiler, name: &str) -> CompileResult<()> {
        let id = self.into_var()?;
        let var = c.scopes.var_mut(self.span, id)?;
        var.declared = true;
        c.scopes.named_with_id(name, id, self.span)?;
        Ok(())
    }

    /// Tro to convert into var.
    pub(crate) fn try_into_var(self) -> Option<VarId> {
        match self.kind {
            ValueKind::Var(id) => Some(id),
            _ => None,
        }
    }

    /// Forcibly convert into var.
    fn into_var(self) -> CompileResult<VarId> {
        match self.kind {
            ValueKind::Unreachable => {
                return Err(CompileError::msg(
                    self.span,
                    "tried to use an unreachable value",
                ));
            }
            ValueKind::Empty => {
                return Err(CompileError::msg(self.span, "tried to use an empty value"))
            }
            ValueKind::Var(id) => Ok(id),
        }
    }
}

/// The kind of a stack value.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ValueKind {
    /// No value produced.
    Empty,
    /// Value is not reachable.
    Unreachable,
    /// Result belongs to the the specified variable.
    Var(VarId),
}

/// Compiler trait implemented for things that can be compiled.
///
/// This is the new compiler trait to implement.
pub(crate) trait Assemble {
    /// Walk the current type with the given item.
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value>;
}

/// Assemble a constant.
pub(crate) trait AssembleConst {
    fn assemble_const(&self, c: &mut Compiler<'_>, needs: Needs, span: Span) -> CompileResult<()>;
}

/// Assemble a function.
pub(crate) trait AssembleFn {
    /// Walk the current type with the given item.
    fn assemble_fn(&self, c: &mut Compiler<'_>, instance_fn: bool) -> CompileResult<()>;
}

/// Assemble a closure with captures.
pub(crate) trait AssembleClosure {
    fn assemble_closure(
        &self,
        c: &mut Compiler<'_>,
        captures: &[CompileMetaCapture],
    ) -> CompileResult<()>;
}
