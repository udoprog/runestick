use crate::compiling::assemble::prelude::*;

/// Compile an `.await` expression.
impl Assemble for ast::ExprAssign {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprAssign => {:?}", c.source.source(span));

        let supported = match &self.lhs {
            // <var> = <value>
            ast::Expr::Path(path) => {
                let span = path.span();

                if let Some(ident) = path.try_as_ident() {
                    let guard = c.scopes.push_child(span)?;
                    let target = self.rhs.assemble(c, Needs::Value)?.apply_targeted(c)?;
                    let ident = ident.resolve(c.storage, &*c.source)?;

                    let var = c
                        .scopes
                        .get_var_mut(&*ident, c.source_id, c.visitor, span)?;

                    match target {
                        InstAddress::Top => {
                            c.asm.push(Inst::Replace { offset: var.offset }, span);
                        }
                        InstAddress::Offset(offset) => {
                            var.offset = offset;
                        }
                    }

                    c.scopes.pop(guard, span)?;
                    true
                } else {
                    false
                }
            }
            // <expr>.<field> = <value>
            ast::Expr::FieldAccess(field_access) => {
                let span = field_access.span();

                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Ident(index) => {
                        let slot = index.resolve(c.storage, &*c.source)?;
                        let slot = c.unit.new_static_string(index, slot.as_ref())?;

                        self.rhs.assemble(c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(self.rhs.span())?;

                        field_access.expr.assemble(c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(span)?;

                        c.asm.push(Inst::ObjectIndexSet { slot }, span);
                        c.scopes.undecl_anon(span, 2)?;
                        true
                    }
                    ast::ExprField::LitNumber(field) => {
                        let number = field.resolve(c.storage, &*c.source)?;
                        let index = number.as_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        self.rhs.assemble(c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(self.rhs.span())?;

                        field_access.expr.assemble(c, Needs::Value)?.apply(c)?;
                        c.asm.push(Inst::TupleIndexSet { index }, span);
                        c.scopes.undecl_anon(span, 1)?;
                        true
                    }
                }
            }
            ast::Expr::Index(expr_index_get) => {
                let span = expr_index_get.span();
                log::trace!("ExprIndexSet => {:?}", c.source.source(span));

                self.rhs.assemble(c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;

                expr_index_get.target.assemble(c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;

                expr_index_get.index.assemble(c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;

                c.asm.push(Inst::IndexSet, span);
                c.scopes.undecl_anon(span, 3)?;
                true
            }
            _ => false,
        };

        if !supported {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedAssignExpr,
            ));
        }

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        Ok(Asm::top(span))
    }
}
