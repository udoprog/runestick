use crate::compiling::compile::prelude::*;

/// Compile a select expression.
impl Compile<(&ast::ExprSelect, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_select, needs): (&ast::ExprSelect, Needs)) -> CompileResult<()> {
        let span = expr_select.span();
        log::trace!("ExprSelect => {:?}", self.source.source(span));
        let len = expr_select.branches.len();
        self.contexts.push(span);

        let mut default_branch = None;
        let mut branches = Vec::new();

        let end_label = self.asm.new_label("select_end");

        for (branch, _) in &expr_select.branches {
            match branch {
                ast::ExprSelectBranch::Pat(pat) => {
                    let label = self.asm.new_label("select_branch");
                    branches.push((label, pat));
                }
                ast::ExprSelectBranch::Default(def) => {
                    if default_branch.is_some() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::SelectMultipleDefaults,
                        ));
                    }

                    let label = self.asm.new_label("select_default");
                    default_branch = Some((def, label));
                }
            }
        }

        for (_, branch) in &branches {
            self.compile((&*branch.expr, Needs::Value))?;
        }

        self.asm.push(Inst::Select { len }, span);

        for (branch, (label, _)) in branches.iter().enumerate() {
            self.asm.jump_if_branch(branch as i64, *label, span);
        }

        if let Some((_, label)) = &default_branch {
            self.asm.push(Inst::Pop, span);
            self.asm.jump(*label, span);
        }

        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.asm.jump(end_label, span);

        for (label, branch) in branches {
            let span = branch.span();
            self.asm.label(label)?;

            let expected = self.scopes.push_child(span)?;

            // NB: loop is actually useful.
            #[allow(clippy::never_loop)]
            loop {
                match &branch.pat {
                    ast::Pat::PatPath(path) => {
                        let (_, item) = self.convert_path_to_named(&path.path)?;

                        if let Some(local) = item.as_local() {
                            self.scopes.decl_var(local, path.span())?;
                            break;
                        }
                    }
                    ast::Pat::PatIgnore(..) => {
                        self.asm.push(Inst::Pop, span);
                        break;
                    }
                    _ => (),
                }

                return Err(CompileError::new(
                    branch,
                    CompileErrorKind::UnsupportedSelectPattern,
                ));
            }

            // Set up a new scope with the binding.
            self.compile((&*branch.body, needs))?;
            self.clean_last_scope(span, expected, needs)?;
            self.asm.jump(end_label, span);
        }

        if let Some((branch, label)) = default_branch {
            self.asm.label(label)?;
            self.compile((&*branch.body, needs))?;
        }

        self.asm.label(end_label)?;

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal(&span, "missing parent context"))?;

        Ok(())
    }
}
