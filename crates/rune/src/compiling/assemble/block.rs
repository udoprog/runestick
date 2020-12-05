use crate::compiling::assemble::prelude::*;

/// Compile an async block.
impl AssembleClosure for ast::Block {
    fn assemble_closure(
        &self,
        c: &mut Compiler<'_>,
        captures: &[CompileMetaCapture],
    ) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBlock (procedure) => {:?}", c.source.source(span));

        let guard = c.scopes.push();

        for capture in captures {
            c.scopes.named(&capture.ident, span)?;
        }

        let value = self.assemble(c, Needs::Value)?;
        c.locals_clean(span, value)?;
        let scope = guard.transfer(span, c, value)?;
        debug_assert!(scope.is_empty());

        c.asm.push(Inst::Return, span);
        Ok(())
    }
}

/// Call a block.
impl Assemble for ast::Block {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("Block => {:?}", c.source.source(span));

        c.contexts.push(span);
        let guard = c.scopes.push();

        let mut last = None::<(&ast::Expr, bool)>;

        for stmt in &self.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Local(local) => {
                    if let Some((stmt, _)) = std::mem::take(&mut last) {
                        // NB: terminated expressions do not need to produce a value.
                        stmt.assemble(c, Needs::None)?.ignore(c)?;
                    }

                    local.assemble(c, Needs::None)?.ignore(c)?;
                    continue;
                }
                ast::Stmt::Expr(expr, semi) => (expr, semi.is_some()),
                ast::Stmt::Item(..) => continue,
            };

            if let Some((stmt, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                // NB: terminated expressions do not need to produce a value.
                stmt.assemble(c, Needs::None)?;
            }
        }

        let value = if let Some((expr, term)) = last {
            if term {
                expr.assemble(c, Needs::None)?
            } else {
                expr.assemble(c, needs)?
            }
        } else {
            Value::empty(span)
        };

        let value = if !value.is_present() && needs.value() {
            c.asm.push(Inst::unit(), span);
            Value::unnamed(span, c)
        } else {
            value
        };

        c.locals_clean(span, value)?;
        let scope = guard.transfer(span, c, value)?;
        debug_assert!(scope.is_empty());

        c.contexts
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

        Ok(value)
    }
}
