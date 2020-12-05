use crate::compiling::assemble::prelude::*;

/// Compile an expression.
impl Assemble for ast::ExprIndex {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprIndex => {:?}", c.source.source(span));

        let target = self.target.assemble(c, Needs::Value)?.address(c)?;
        let index = self.index.assemble(c, Needs::Value)?.address(c)?;

        c.asm.push(Inst::IndexGet { index, target }, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
