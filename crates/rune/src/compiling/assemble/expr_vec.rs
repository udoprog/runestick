use crate::compiling::assemble::prelude::*;

/// Compile a literal vector.
impl Assemble for ast::ExprVec {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprVec => {:?}", c.source.source(span));

        let count = self.items.len();

        let mut values = Vec::new();

        for (expr, _) in &self.items {
            values.push(expr.assemble(c, Needs::Value)?);
        }

        for value in values {
            value.pop(c)?;
        }

        c.asm.push(Inst::Vec { count }, span);

        // Evaluate the expressions one by one, then pop them to cause any
        // side effects (without creating an object).
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
