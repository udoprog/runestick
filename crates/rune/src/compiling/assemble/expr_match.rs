use crate::compiling::assemble::prelude::*;

impl Assemble for ast::ExprMatch {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprMatch => {:?}", c.source.source(span));

        let guard = c.scopes.push();
        let expr_var = self.expr.assemble(c, Needs::Value)?;

        let end_label = c.asm.new_label("match_end");
        let mut branches = Vec::new();

        for (branch, _) in &self.branches {
            let span = branch.span();

            let branch_label = c.asm.new_label("match_branch");
            let match_false = c.asm.new_label("match_false");

            let guard = c.scopes.push();

            let load = move |_: &mut Compiler, _: Needs| Ok(expr_var);

            c.compile_pat(&branch.pat, match_false, &load)?;

            if let Some((_, condition)) = &branch.condition {
                let span = condition.span();

                condition.assemble(c, Needs::Value)?.pop(c)?;

                c.asm
                    .pop_and_jump_if_not(c.scopes.locals(), match_false, span);

                c.asm.jump(branch_label, span);
            }

            c.asm.jump(branch_label, span);
            c.asm.label(match_false)?;

            let scope = guard.pop(span, c)?;
            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        c.asm.jump(end_label, span);

        let mut it = self.branches.iter().zip(branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            c.asm.label(label)?;

            let guard = c.scopes.push_scope(span, scope)?;
            let value = branch.body.assemble(c, needs)?;
            c.locals_clean(span, value)?;
            let scope = guard.pop(span, c)?;
            debug_assert!(scope.is_empty(), "scope used in a branch should be empty");

            if it.peek().is_some() {
                c.asm.jump(end_label, span);
            }
        }

        // Clean up temp loop variable.
        c.locals_clean(span, Value::empty(span))?;
        guard.pop(span, c)?;
        c.asm.label(end_label)?;

        if !needs.value() {
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
