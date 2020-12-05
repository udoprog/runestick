use crate::compiling::assemble::prelude::*;

/// Compile a for loop.
impl Assemble for ast::ExprFor {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprFor => {:?}", c.source.source(span));

        let continue_label = c.asm.new_label("for_continue");
        let end_label = c.asm.new_label("for_end");
        let break_label = c.asm.new_label("for_break");

        let break_var_count = c.scopes.totals();

        let iter_var = {
            self.iter.assemble(c, Needs::Value)?.pop(c)?;

            let iter_var = c.scopes.unnamed(span);
            c.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *runestick::Protocol::INTO_ITER,
                    args: 0,
                },
                span,
                format!("into_iter (offset: {})", iter_var),
            );

            Value::var(span, iter_var)
        };

        let binding_span = self.binding.span();

        // Declare named loop variable.
        let binding_var = {
            c.asm.push(Inst::unit(), self.iter.span());
            Value::unnamed(binding_span, c)
        };

        // Declare storage for memoized `next` instance fn.
        let next = if c.options.memoize_instance_fn {
            let span = self.iter.span();

            // Declare the named loop variable and put it in the scope.
            iter_var.copy(c)?;

            c.asm.push_with_comment(
                Inst::LoadInstanceFn {
                    hash: *runestick::Protocol::NEXT,
                },
                span,
                "load instance fn (memoize)",
            );

            Some(Value::unnamed(span, c))
        } else {
            None
        };

        let continue_var_count = c.scopes.totals();
        c.asm.label(continue_label)?;

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            continue_label,
            continue_var_count,
            break_label,
            break_var_count,
            needs,
            drop: Some(iter_var),
        });

        // Use the memoized loop variable.
        if let Some(next) = next {
            iter_var.copy(c)?;
            next.copy(c)?;

            c.asm.push(Inst::CallFn { args: 1 }, span);

            let offset = binding_var.offset(c)?;
            c.asm.push(Inst::Replace { offset }, binding_span);
        } else {
            // call the `next` function to get the next level of iteration, bind the
            // result to the loop variable in the loop.
            iter_var.copy(c)?;

            c.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *runestick::Protocol::NEXT,
                    args: 0,
                },
                span,
                "next",
            );

            let offset = binding_var.offset(c)?;
            c.asm.push(Inst::Replace { offset }, binding_span);
        }

        // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
        let offset = binding_var.offset(c)?;
        c.asm.iter_next(offset, end_label, binding_span);

        let guard = c.scopes.push();

        c.compile_pat_offset(&self.binding, binding_var)?;

        self.body.assemble(c, Needs::None)?.ignore(c)?;

        c.locals_clean(span, Needs::None)?;
        guard.pop(span, c)?;

        c.asm.jump(continue_label, span);
        c.asm.label(end_label)?;

        // Drop the iterator.
        let offset = iter_var.offset(c)?;
        c.asm.push(Inst::Drop { offset }, span);

        c.locals_clean(span, Needs::None)?;

        // NB: breaks produce their own value.
        c.asm.label(break_label)?;

        // NB: If a value is needed from a for loop, encode it as a unit.
        if needs.value() {
            return Ok(Value::empty(span));
        }

        c.asm.push(Inst::unit(), span);
        Ok(Value::unnamed(span, c))
    }
}
