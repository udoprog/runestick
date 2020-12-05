use crate::compiling::assemble::prelude::*;

/// Compile a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
impl Assemble for ast::ExprBreak {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprBreak => {:?}", c.source.source(span));

        let current_loop = match c.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::BreakOutsideOfLoop,
                ));
            }
        };

        let (last_loop, to_drop, value) = if let Some(expr) = &self.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    let value = expr.assemble(c, current_loop.needs)?;
                    (
                        current_loop,
                        current_loop.drop.into_iter().collect(),
                        Some(value),
                    )
                }
                ast::ExprBreakValue::Label(label) => {
                    let (last_loop, to_drop) =
                        c.loops.walk_until_label(c.storage, &*c.source, *label)?;
                    (last_loop, to_drop, None)
                }
            }
        } else {
            (current_loop, current_loop.drop.into_iter().collect(), None)
        };

        // Drop loop temporary. Typically an iterator.
        for drop in to_drop {
            let offset = drop.offset(c)?;
            c.asm.push(Inst::Drop { offset }, span);
        }

        let vars = c
            .scopes
            .totals()
            .checked_sub(last_loop.break_var_count)
            .ok_or_else(|| CompileError::msg(&span, "var count should be larger"))?;

        if last_loop.needs.value() {
            if let Some(value) = value {
                c.custom_clean(span, value, vars)?;
                value.pop(c)?;
            } else {
                c.custom_clean(span, Value::empty(span), vars)?;
                c.asm.push(Inst::unit(), span);
            }
        } else {
            c.custom_clean(span, Value::empty(span), vars)?;
        }

        c.asm.jump(last_loop.break_label, span);
        Ok(Value::unreachable(span))
    }
}
