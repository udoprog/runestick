use crate::compiling::assemble::prelude::*;

/// Compile a return.
impl Assemble for ast::ExprReturn {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprReturn => {:?}", c.source.source(span));

        // NB: drop any loop temporaries.
        for l in c.loops.iter() {
            if let Some(drop) = l.drop {
                let offset = drop.offset(c)?;
                c.asm.push(Inst::Drop { offset }, span);
            }
        }

        if let Some(expr) = &self.expr {
            let value = expr.assemble(c, Needs::Value)?;
            c.custom_clean(span, value, c.scopes.totals())?;
            value.pop(c)?;
            c.asm.push(Inst::Return, span);
        } else {
            c.custom_clean(span, Value::empty(span), c.scopes.totals())?;
            c.asm.push(Inst::ReturnUnit, span);
        }

        Ok(Value::unreachable(span))
    }
}
