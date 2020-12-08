use crate::compiling::v2::assemble::prelude::*;

/// Assembler for a block.
impl Assemble for ast::Block {
    fn assemble(&self, block: &Block, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Block => {:?}", c.source.source(span));

        c.contexts.push(span);
        c.scope.push();

        let mut last = None;

        for stmt in &self.statements {
            let (next, semi) = match stmt {
                ast::Stmt::Local(..) => {
                    continue;
                }
                ast::Stmt::Expr(expr, semi) => (expr.assemble(&block, c)?, semi),
                ast::Stmt::Item(..) => continue,
            };

            // Force a use of the last evaluated value in case it's replaced.
            if let Some(last) = last.take() {
                block.use_(last);
            }

            // NB: semi-colons were checked during parsing.
            if semi.is_some() {
                block.use_(next);
            } else {
                last = Some(next);
            }
        }

        c.scope.pop(span)?;
        Ok(last.unwrap_or_else(|| block.unit()))
    }
}
