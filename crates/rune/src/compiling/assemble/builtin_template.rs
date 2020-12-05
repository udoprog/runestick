use crate::compiling::assemble::prelude::*;
use crate::query::BuiltInTemplate;

/// Compile a literal template string.
impl Assemble for BuiltInTemplate {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span;
        log::trace!("BuiltInTemplate => {:?}", c.source.source(span));

        let guard = c.scopes.push();
        let mut size_hint = 0;
        let mut expansions = 0;

        let mut values = Vec::new();

        for expr in &self.exprs {
            if let ast::Expr::Lit(expr_lit) = expr {
                if let ast::ExprLit {
                    lit: ast::Lit::Str(s),
                    ..
                } = &**expr_lit
                {
                    let s = s.resolve_template_string(&c.storage, &c.source)?;
                    size_hint += s.len();

                    let slot = c.unit.new_static_string(span, &s)?;
                    c.asm.push(Inst::String { slot }, span);
                    c.scopes.unnamed(span);
                    continue;
                }
            }

            expansions += 1;
            values.push(expr.assemble(c, Needs::Value)?);
        }

        if self.from_literal && expansions == 0 {
            c.warnings
                .template_without_expansions(c.source_id, span, c.context());
        }

        for value in values {
            value.pop(c)?;
        }

        c.asm.push(
            Inst::StringConcat {
                len: self.exprs.len(),
                size_hint,
            },
            span,
        );

        guard.pop(span, c)?;

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
