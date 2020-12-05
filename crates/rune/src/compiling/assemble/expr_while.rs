use crate::compiling::assemble::prelude::*;

/// Compile a while loop.
impl Assemble for ast::ExprWhile {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprWhile => {:?}", c.source.source(span));

        let continue_label = c.asm.new_label("while_continue");
        let then_label = c.asm.new_label("whiel_then");
        let end_label = c.asm.new_label("while_end");
        let break_label = c.asm.new_label("while_break");

        let var_count = c.scopes.totals();

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            continue_label,
            continue_var_count: var_count,
            break_label,
            break_var_count: var_count,
            needs,
            drop: None,
        });

        c.asm.label(continue_label)?;

        let then_scope = c.compile_condition(&self.condition, then_label)?;
        let guard = c.scopes.push_scope(span, then_scope)?;

        c.asm.jump(end_label, span);
        c.asm.label(then_label)?;

        self.body.assemble(c, Needs::None)?.ignore(c)?;

        c.asm.jump(continue_label, span);
        c.asm.label(end_label)?;

        guard.pop(span, c)?;

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        // NB: breaks produce their own value / perform their own cleanup.
        c.asm.label(break_label)?;

        Ok(if !needs.value() {
            Value::empty(span)
        } else {
            Value::unnamed(span, c)
        })
    }
}
