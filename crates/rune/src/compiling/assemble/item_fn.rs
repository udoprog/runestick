use crate::compiling::assemble::prelude::*;

impl AssembleFn for ast::ItemFn {
    fn assemble_fn(&self, c: &mut Compiler<'_>, instance_fn: bool) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", c.source.source(span));

        let mut patterns = Vec::new();
        let mut first = true;

        for (arg, _) in &self.args {
            let span = arg.span();

            match arg {
                ast::FnArg::SelfValue(s) => {
                    if !instance_fn || !first {
                        return Err(CompileError::new(span, CompileErrorKind::UnsupportedSelf));
                    }

                    let span = s.span();
                    c.scopes.named("self", span)?;
                }
                ast::FnArg::Pat(pat) => {
                    let value = Value::unnamed(pat.span(), c);
                    patterns.push((pat, value));
                }
            }

            first = false;
        }

        for (pat, value) in patterns {
            c.compile_pat_offset(pat, value)?;
        }

        if self.body.statements.is_empty() {
            c.locals_clean(span, Needs::None)?;
            c.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        if !self.body.produces_nothing() {
            self.body.assemble(c, Needs::Value)?;
            c.locals_clean(span, Needs::Value)?;
            c.asm.push(Inst::Return, span);
        } else {
            self.body.assemble(c, Needs::None)?;
            c.locals_clean(span, Needs::None)?;
            c.asm.push(Inst::ReturnUnit, span);
        }

        c.scopes.pop_last(span)?;
        Ok(())
    }
}
