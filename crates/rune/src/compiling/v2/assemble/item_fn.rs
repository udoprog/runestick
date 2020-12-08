use crate::compiling::v2::assemble::prelude::*;

impl AssembleFn for ast::ItemFn {
    fn assemble_fn(
        &self,
        c: &mut Compiler<'_>,
        instance_fn: bool,
    ) -> CompileResult<rune_ssa::Block> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", c.source.source(span));

        let block = c.sm.block();

        let mut first = true;

        for (arg, _) in &self.args {
            let span = arg.span();
            let value = block.input();
            let first = std::mem::take(&mut first);

            match arg {
                ast::FnArg::SelfValue(s) => {
                    let span = s.span();

                    if !instance_fn || !first {
                        return Err(CompileError::new(span, CompileErrorKind::UnsupportedSelf));
                    }

                    c.scope.declare(span, "self", value)?;
                    continue;
                }
                ast::FnArg::Pat(ast::Pat::PatPath(path)) => {
                    if let Some(ident) = path.path.try_as_ident() {
                        let name = c.resolve(ident)?;
                        c.scope.declare(span, &name, value)?;
                        continue;
                    } else {
                        return Err(CompileError::msg(span, "path not supported yet"));
                    }
                }
                _ => {
                    return Err(CompileError::msg(span, "argument not supported yet"));
                }
            }
        }

        let value = self.body.assemble(&block, c)?;
        block.return_(value);
        Ok(block)
    }
}
