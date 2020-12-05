use crate::collections::HashMap;
use crate::compiling::{Assembly, Compiler};
use crate::{CompileError, CompileErrorKind, CompileResult, CompileVisitor};
use runestick::{Inst, SourceId, Span};
use std::fmt;

/// The identifier of a variable.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(usize);

impl fmt::Debug for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A calculated variable offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarOffset {
    /// Variable is on the top of the stack.
    Top,
    /// Variable is at the given offset.
    Offset(usize),
}

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
#[derive(Clone, Copy)]
pub struct Var {
    /// Slot offset from the current stack frame.
    pub(crate) offset: usize,
    /// Token assocaited with the variable.
    pub(crate) span: Span,
    /// Variable has been taken at the given position.
    moved_at: Option<Span>,
}

impl fmt::Debug for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {:?})", self.offset, self.span)
    }
}

impl Var {
    /// Copy the declared variable.
    pub(crate) fn copy<C>(&self, asm: &mut Assembly, span: Span, comment: C)
    where
        C: AsRef<str>,
    {
        asm.push_with_comment(
            Inst::Copy {
                offset: self.offset,
            },
            span,
            comment,
        );
    }

    /// Move the declared variable.
    pub(crate) fn do_move<C>(&self, asm: &mut Assembly, span: Span, comment: C)
    where
        C: AsRef<str>,
    {
        asm.push_with_comment(
            Inst::Move {
                offset: self.offset,
            },
            span,
            comment,
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Scope {
    /// Named variables.
    locals: HashMap<String, VarId>,
    /// At which point on the stack the head of this scope is.
    head: usize,
    /// Vars that were popped as part of this scope. These are only populated
    /// once the scope is popped.
    stack: Vec<Var>,
}

impl Scope {
    /// Construct a new locals handlers.
    fn new() -> Scope {
        Self {
            locals: HashMap::new(),
            head: 0,
            stack: vec![],
        }
    }

    /// Test the length of the scope.
    pub(crate) fn len(&self) -> usize {
        self.stack.len()
    }

    /// Test if the scope is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct Scopes {
    /// Scopes with named locals.
    scopes: Vec<Scope>,
    /// Stack variables.
    stack: Vec<Var>,
}

impl Scopes {
    /// Construct a new collection of scopes.
    pub(crate) fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
            stack: Vec::new(),
        }
    }

    /// Construct a new scope based on the current stack location.
    pub(crate) fn scope(&self) -> Scope {
        Scope {
            locals: HashMap::new(),
            head: self.stack.len(),
            stack: Vec::new(),
        }
    }

    /// Clean the current scope wit the given count.
    pub(crate) fn pop(&mut self, span: Span, count: usize) -> CompileResult<()> {
        let head = self.last(span)?.head;

        let new_top = self
            .stack
            .len()
            .checked_sub(count)
            .ok_or_else(|| CompileError::msg(span, "pop out of bounds for scope"))?;

        if new_top < head {
            return Err(CompileError::msg(span, "pop out of bounds for scope head"));
        }

        for _ in 0..count {
            self.stack
                .pop()
                .ok_or_else(|| CompileError::msg(span, "stack pop is out of bounds"))?;
        }

        Ok(())
    }

    /// Try to get the given variable with its ID included.
    pub(crate) fn try_get_var_with_id(
        &self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<Option<(&Var, VarId)>> {
        for scope in self.scopes.iter().rev() {
            let id = match scope.locals.get(name).copied() {
                Some(id) => id,
                None => continue,
            };

            let var = self
                .stack
                .get(id.0)
                .ok_or_else(|| CompileError::new(span, CompileErrorKind::VarIdMissing { id }))?;

            if let Some(moved_at) = var.moved_at {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::VariableMoved { moved_at },
                ));
            }

            log::trace!("found var: {} => {:?}", name, var);
            visitor.visit_variable_use(source_id, var, span);
            return Ok(Some((var, id)));
        }

        Ok(None)
    }

    /// Try to get the local with the given name. Returns `None` if it's
    /// missing.
    pub(crate) fn try_get_var(
        &self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<Option<&Var>> {
        let result = self.try_get_var_with_id(name, source_id, visitor, span)?;

        if let Some((var, _)) = result {
            Ok(Some(var))
        } else {
            Ok(None)
        }
    }

    /// Try to get the local with the given name. Returns `None` if it's
    /// missing.
    pub(crate) fn try_get_var_mut(
        &mut self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<Option<&mut Var>> {
        log::trace!("get var: {}", name);

        for scope in self.scopes.iter_mut().rev() {
            let id = match scope.locals.get(name).copied() {
                Some(id) => id,
                None => continue,
            };

            let var = self
                .stack
                .get_mut(id.0)
                .ok_or_else(|| CompileError::new(span, CompileErrorKind::VarIdMissing { id }))?;

            if let Some(moved_at) = var.moved_at {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::VariableMoved { moved_at },
                ));
            }

            log::trace!("found var: {} => {:?}", name, var);
            visitor.visit_variable_use(source_id, var, span);
            return Ok(Some(var));
        }

        Ok(None)
    }

    /// Try to take the local with the given name. Returns `None` if it's
    /// missing.
    pub(crate) fn try_take_var(
        &mut self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<Option<&Var>> {
        log::trace!("get var: {}", name);

        for scope in self.scopes.iter_mut().rev() {
            let id = match scope.locals.get(name).copied() {
                Some(id) => id,
                None => continue,
            };

            let var = self
                .stack
                .get_mut(id.0)
                .ok_or_else(|| CompileError::new(span, CompileErrorKind::VarIdMissing { id }))?;

            if let Some(moved_at) = var.moved_at {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::VariableMoved { moved_at },
                ));
            }

            var.moved_at = Some(span);

            log::trace!("found var: {} => {:?}", name, var);
            visitor.visit_variable_use(source_id, var, span);
            return Ok(Some(var));
        }

        Ok(None)
    }

    /// Get the local with the given name.
    pub(crate) fn get_var(
        &self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<&Var> {
        match self.try_get_var(name, source_id, visitor, span)? {
            Some(var) => Ok(var),
            None => Err(CompileError::new(
                span,
                CompileErrorKind::MissingLocal {
                    name: name.to_owned(),
                },
            )),
        }
    }

    /// Get the local with the given variable mutably.
    pub(crate) fn get_var_mut(
        &mut self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<&mut Var> {
        match self.try_get_var_mut(name, source_id, visitor, span)? {
            Some(var) => Ok(var),
            None => Err(CompileError::new(
                span,
                CompileErrorKind::MissingLocal {
                    name: name.to_owned(),
                },
            )),
        }
    }

    /// Take the local with the given name.
    pub(crate) fn take_var(
        &mut self,
        name: &str,
        source_id: SourceId,
        visitor: &mut dyn CompileVisitor,
        span: Span,
    ) -> CompileResult<&Var> {
        match self.try_take_var(name, source_id, visitor, span)? {
            Some(var) => Ok(var),
            None => Err(CompileError::new(
                span,
                CompileErrorKind::MissingLocal {
                    name: name.to_owned(),
                },
            )),
        }
    }

    /// Pop the stack.
    pub(crate) fn stack_pop(&mut self, span: Span) -> CompileResult<(Var, VarId)> {
        let head = self.last(span)?.head;

        if self.stack.len() == head {
            return Err(CompileError::msg(span, "stack pop is out of bounds"));
        }

        let var = self
            .stack
            .pop()
            .ok_or_else(|| CompileError::msg(span, "stack pop is out of bounds"))?;

        let id = VarId(self.stack.len());
        Ok((var, id))
    }

    /// Push a custom variable onto the stack.
    pub(crate) fn stack_push(&mut self, var: Var) -> VarId {
        let id = VarId(self.stack.len());
        self.stack.push(var);
        id
    }

    /// Declare a new unnamed variable.
    pub(crate) fn unnamed(&mut self, span: Span) -> VarId {
        let offset = self.stack.len();
        self.unnamed_with_offset(span, offset)
    }

    /// Declare a new unnamed variable with the specified offset.
    pub(crate) fn unnamed_with_offset(&mut self, span: Span, offset: usize) -> VarId {
        let id = VarId(self.stack.len());

        let local = Var {
            offset,
            span,
            moved_at: None,
        };

        self.stack.push(local);
        id
    }

    /// Insert a new named local variable.
    pub(crate) fn named(&mut self, name: &str, span: Span) -> CompileResult<VarId> {
        let id = self.unnamed(span);
        let scope = self.last_mut(span)?;

        if let Some(old) = scope.locals.insert(name.to_owned(), id) {
            let old = self.stack.get(old.0).ok_or_else(|| {
                CompileError::new(span, CompileErrorKind::VarIdMissing { id: old })
            })?;

            return Err(CompileError::new(
                span,
                CompileErrorKind::VariableConflict {
                    name: name.to_owned(),
                    existing_span: old.span,
                },
            ));
        }

        Ok(id)
    }

    /// Insert a new named local variable with a specific id.
    pub(crate) fn named_with_id(
        &mut self,
        name: &str,
        id: VarId,
        span: Span,
    ) -> CompileResult<VarId> {
        let scope = self.last_mut(span)?;

        if let Some(old) = scope.locals.insert(name.to_owned(), id) {
            let old = self.stack.get(old.0).ok_or_else(|| {
                CompileError::new(span, CompileErrorKind::VarIdMissing { id: old })
            })?;

            return Err(CompileError::new(
                span,
                CompileErrorKind::VariableConflict {
                    name: name.to_owned(),
                    existing_span: old.span,
                },
            ));
        }

        Ok(id)
    }

    /// Set the variable with the given name, with the given offset.
    pub(crate) fn set_with_offset(
        &mut self,
        name: &str,
        offset: usize,
        span: Span,
    ) -> CompileResult<VarId> {
        let id = VarId(self.stack.len());
        self.last_mut(span)?.locals.insert(name.to_owned(), id);

        self.stack.push(Var {
            offset,
            span,
            moved_at: None,
        });

        Ok(id)
    }

    /// Set the value of the given name.
    pub(crate) fn set(&mut self, name: &str, span: Span) -> CompileResult<VarId> {
        let offset = self.stack.len();
        self.set_with_offset(name, offset, span)
    }

    /// Construct a new child scope and return its guard.
    ///
    /// This is realted to advanced scope management, where a previously created
    /// scope can be added back to the stack.
    pub(crate) fn push_scope(&mut self, span: Span, mut scope: Scope) -> CompileResult<ScopeGuard> {
        if scope.head != self.stack.len() {
            return Err(CompileError::msg(
                span,
                "pushed scope head does not match length of scope",
            ));
        }

        self.stack.extend(scope.stack.drain(..));
        self.scopes.push(scope);

        log::trace!(">> scope guard: {}", self.scopes.len());

        Ok(ScopeGuard {
            expected: self.scopes.len(),
        })
    }

    /// Construct a new child scope.
    pub(crate) fn push(&mut self) -> ScopeGuard {
        let scope = self.scope();
        self.scopes.push(scope);

        log::trace!(">> scope guard: {}", self.scopes.len());

        ScopeGuard {
            expected: self.scopes.len(),
        }
    }

    /// Pop the last scope and compare with the expected length.
    pub(crate) fn pop_last(&mut self, span: Span) -> CompileResult<Scope> {
        if self.scopes.len() != 1 {
            return Err(CompileError::msg(span, "missing last scope"));
        }

        let scope = self
            .scopes
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing last scope"))?;

        Ok(scope)
    }

    /// Gets the stack offset for the given variable id.
    pub(crate) fn offset_of(&self, span: Span, id: VarId) -> CompileResult<VarOffset> {
        let var = self.var_of(span, id)?;

        Ok(if var.offset + 1 == self.stack.len() {
            VarOffset::Top
        } else {
            VarOffset::Offset(var.offset)
        })
    }

    /// Gets the stack offset for the given variable id.
    pub(crate) fn var_of(&self, span: Span, id: VarId) -> CompileResult<&Var> {
        self.stack
            .get(id.0)
            .ok_or_else(|| CompileError::new(span, CompileErrorKind::VarIdMissing { id }))
    }

    /// Total stack size.
    pub(crate) fn totals(&self) -> usize {
        self.stack.len()
    }

    /// Get the number of variables local to the current scope.
    pub(crate) fn locals(&self) -> usize {
        if let Some(scope) = self.scopes.last() {
            self.stack.len() - scope.head
        } else {
            self.stack.len()
        }
    }

    /// Get the local with the given name.
    fn last(&self, span: Span) -> CompileResult<&Scope> {
        Ok(self
            .scopes
            .last()
            .ok_or_else(|| CompileError::msg(&span, "missing head of locals"))?)
    }

    /// Get the last locals scope.
    fn last_mut(&mut self, span: Span) -> CompileResult<&mut Scope> {
        Ok(self
            .scopes
            .last_mut()
            .ok_or_else(|| CompileError::msg(&span, "missing head of locals"))?)
    }
}

/// A scope guard.
#[must_use = "must be consumed with `pop()`"]
pub struct ScopeGuard {
    expected: usize,
}

impl ScopeGuard {
    /// Popping of the scope guard.
    pub(crate) fn pop(self, span: Span, c: &mut Compiler<'_>) -> CompileResult<Scope> {
        let expected = std::mem::ManuallyDrop::new(self).expected;

        log::trace!("<< scope guard: {}", expected);

        if c.scopes.scopes.len() != expected {
            log::trace!(
                "scope guard mismatch: {} != {}",
                c.scopes.stack.len(),
                expected
            );
            return Err(CompileError::msg(span, "scope guard mismatch"));
        }

        let mut scope = c
            .scopes
            .scopes
            .pop()
            .ok_or_else(|| CompileError::msg(span, "no scopes to pop"))?;

        scope.stack = c.scopes.stack.drain(scope.head..).collect();
        Ok(scope)
    }

    /// Pop and transfer the given number of variables from this scope.
    pub(crate) fn transfer(
        self,
        span: Span,
        c: &mut Compiler<'_>,
        transfer: usize,
    ) -> CompileResult<Scope> {
        let mut scope = self.pop(span, c)?;

        if scope.stack.len() < transfer {
            return Err(CompileError::msg(
                span,
                "not enough variables in popped scope to transfer",
            ));
        }

        c.scopes.stack.extend(scope.stack.drain(..transfer));
        Ok(scope)
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if cfg!(debug) {
            panic!("scope guard at level {} was not disarmed", self.expected)
        }
    }
}
