use crate::compiling::assemble::prelude::*;

/// Compile a loop.
impl Assemble for ast::ExprLoop {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprLoop => {:?}", c.source.source(span));

        let continue_label = c.asm.new_label("loop_continue");
        let break_label = c.asm.new_label("loop_break");

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
        self.body.assemble(c, Needs::None)?.ignore(c)?;
        c.asm.jump(continue_label, span);
        c.asm.label(break_label)?;

        if !needs.value() {
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
