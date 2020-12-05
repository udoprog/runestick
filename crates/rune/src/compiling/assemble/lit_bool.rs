use crate::compiling::assemble::prelude::*;

/// Compile a literal boolean such as `true`.
impl Assemble for ast::LitBool {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("LitBool => {:?}", c.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(Value::empty(span));
        }

        c.asm.push(Inst::bool(self.value), span);
        Ok(Value::unnamed(span, c))
    }
}
