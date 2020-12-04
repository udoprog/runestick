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

use crate::compiling::{CompileError, CompileErrorKind, CompileResult, Compiler, Needs};
use runestick::{CompileMetaCapture, Inst, InstAddress, Span};

#[derive(Debug)]
#[must_use = "must be consumed to make sure the value is realized"]
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

    /// Construct a value that is produced at the top of the stack.
    pub(crate) fn top(span: Span) -> Self {
        Self {
            span,
            kind: ValueKind::Top,
        }
    }

    /// Declare that the assembly resulted in a value in a offset location.
    pub(crate) fn offset(span: Span, offset: usize) -> Self {
        Self {
            span,
            kind: ValueKind::Offset(offset),
        }
    }

    /// Make sure the value is pushed on top of the stack.
    pub(crate) fn push(self, c: &mut Compiler) -> CompileResult<()> {
        match self.kind {
            ValueKind::Empty => {
                return Err(CompileError::new(self.span, CompileErrorKind::ValueEmpty))
            }
            ValueKind::Top => (),
            ValueKind::Offset(offset) => {
                c.asm.push(Inst::Copy { offset }, self.span);
            }
        }

        Ok(())
    }

    /// Ignore the produced value.
    pub(crate) fn ignore(self, c: &mut Compiler) -> CompileResult<()> {
        match self.kind {
            ValueKind::Empty => (),
            ValueKind::Top => {
                c.asm.push(Inst::Pop, self.span);
            }
            ValueKind::Offset(..) => (),
        }

        Ok(())
    }

    /// Assemble into an address.
    ///
    /// # Usage
    ///
    /// In order to use this, you should declare a child scope that you control.
    /// The targeted operation will clean up any values on the stack if it
    /// references e.g. stack top values, but you must make sure that the stack
    /// state in the compiler is balanced by giving it a scope to operate in.
    ///
    /// Clean up is done with:
    ///
    /// ```rust,ignore
    /// let guard = c.scopes.push_child(span)?;
    /// // perform targeted operations.
    /// c.scopes.pop(guard, span)?;
    /// ```
    pub(crate) fn address(self, _: &mut Compiler) -> CompileResult<InstAddress> {
        let address = match self.kind {
            ValueKind::Empty => {
                return Err(CompileError::new(self.span, CompileErrorKind::ValueEmpty))
            }
            ValueKind::Top => InstAddress::Top,
            ValueKind::Offset(offset) => InstAddress::Offset(offset),
        };

        Ok(address)
    }

    /// Declare a variable based on the assembled result.
    pub(crate) fn decl_var(&self, c: &mut Compiler, ident: &str) -> CompileResult<()> {
        match self.kind {
            ValueKind::Empty => {
                return Err(CompileError::new(self.span, CompileErrorKind::ValueEmpty))
            }
            ValueKind::Top => {
                c.scopes.decl_var(ident, self.span)?;
            }
            ValueKind::Offset(offset) => {
                c.scopes.decl_var_with_offset(ident, offset, self.span)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum ValueKind {
    /// No value produced.
    Empty,
    /// Result produced at top of the stack.
    Top,
    /// Result belongs to the the given stack offset.
    Offset(usize),
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
