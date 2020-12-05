use crate::compiling::assemble::prelude::*;

/// Compile a continue expression.
impl Assemble for ast::ExprContinue {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprContinue => {:?}", c.source.source(span));

        let current_loop = match c.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::ContinueOutsideOfLoop,
                ));
            }
        };

        let last_loop = if let Some(label) = &self.label {
            let (last_loop, _) = c.loops.walk_until_label(c.storage, &*c.source, *label)?;
            last_loop
        } else {
            current_loop
        };

        let vars = c
            .scopes
            .totals()
            .checked_sub(last_loop.continue_var_count)
            .ok_or_else(|| CompileError::msg(&span, "var count should be larger"))?;

        c.custom_clean(span, Value::empty(span), vars)?;
        c.asm.jump(last_loop.continue_label, span);
        Ok(Value::unreachable(span))
    }
}
