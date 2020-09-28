use crate::compiling::compile::prelude::*;

/// Compile a literal template string.
impl Compile<(&ast::LitTemplate, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_template, needs): (&ast::LitTemplate, Needs)) -> CompileResult<()> {
        let span = lit_template.span();
        log::trace!("LitTemplate => {:?}", self.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let template = self.query.template_for(lit_template)?.clone();

        if !template.has_expansions {
            self.warnings
                .template_without_expansions(self.source_id, span, self.context());
        }

        let expected = self.scopes.push_child(span)?;

        for c in &template.components {
            match c {
                ast::TemplateComponent::String(string) => {
                    let slot = self.unit.new_static_string(span, &string)?;
                    self.asm.push(Inst::String { slot }, span);
                    self.scopes.decl_anon(span)?;
                }
                ast::TemplateComponent::Expr(expr) => {
                    self.compile((&**expr, Needs::Value))?;
                    self.scopes.decl_anon(span)?;
                }
            }
        }

        self.asm.push(
            Inst::StringConcat {
                len: template.components.len(),
                size_hint: template.size_hint,
            },
            span,
        );

        let _ = self.scopes.pop(expected, span)?;
        Ok(())
    }
}
