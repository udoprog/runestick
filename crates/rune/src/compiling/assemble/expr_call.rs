use crate::compiling::assemble::prelude::*;

/// Compile a call expression.
impl Assemble for ast::ExprCall {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprCall => {:?}", c.source.source(span));

        let args = self.args.len();

        // NB: either handle a proper function call by resolving it's meta hash,
        // or expand the expression.
        #[allow(clippy::never_loop)]
        let path = loop {
            let expr = &self.expr;

            let use_expr = match expr {
                ast::Expr::Path(path) => {
                    log::trace!("ExprCall(Path) => {:?}", c.source.source(span));
                    break path;
                }
                ast::Expr::FieldAccess(expr_field_access) => {
                    if let ast::ExprFieldAccess {
                        expr,
                        expr_field: ast::ExprField::Ident(ident),
                        ..
                    } = &**expr_field_access
                    {
                        log::trace!("ExprCall(ExprFieldAccess) => {:?}", c.source.source(span));

                        let expr = expr.assemble(c, Needs::Value)?;
                        let mut values = Vec::new();

                        for (expr, _) in &self.args {
                            values.push(expr.assemble(c, Needs::Value)?);
                        }

                        for expr in values.into_iter().rev() {
                            expr.pop(c)?;
                        }

                        expr.pop(c)?;

                        let ident = ident.resolve(&c.storage, &*c.source)?;
                        let hash = Hash::instance_fn_name(ident.as_ref());
                        c.asm.push(Inst::CallInstance { hash, args }, span);
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            };

            if use_expr {
                log::trace!("ExprCall(Other) => {:?}", c.source.source(span));

                let mut values = Vec::new();

                for (expr, _) in &self.args {
                    values.push(expr.assemble(c, Needs::Value)?);
                }

                for expr in values.into_iter().rev() {
                    expr.pop(c)?;
                }

                expr.assemble(c, Needs::Value)?.pop(c)?;
                c.asm.push(Inst::CallFn { args }, span);
            }

            if !needs.value() {
                c.asm.push(Inst::Pop, span);
                return Ok(Value::empty(span));
            }

            return Ok(Value::unnamed(span, c));
        };

        let named = c.convert_path_to_named(path)?;

        if let Some(name) = named.as_local() {
            let local = c
                .scopes
                .try_get_var(name, c.source_id, c.visitor, path.span())?
                .copied();

            if let Some(var) = local {
                let mut values = Vec::new();

                for (expr, _) in &self.args {
                    values.push(expr.assemble(c, Needs::Value)?);
                }

                for expr in values.into_iter().rev() {
                    expr.pop(c)?;
                }

                var.copy(&mut c.asm, span, format!("var `{}`", name));
                c.asm.push(Inst::CallFn { args }, span);

                if !needs.value() {
                    c.asm.push(Inst::Pop, span);
                    return Ok(Value::empty(span));
                }

                return Ok(Value::unnamed(span, c));
            }
        }

        let meta = c.lookup_meta(path.span(), &named.item)?;

        match &meta.kind {
            CompileMetaKind::UnitStruct { .. } | CompileMetaKind::UnitVariant { .. } => {
                if 0 != self.args.len() {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedArgumentCount {
                            meta: meta.clone(),
                            expected: 0,
                            actual: self.args.len(),
                        },
                    ));
                }
            }
            CompileMetaKind::TupleStruct { tuple, .. }
            | CompileMetaKind::TupleVariant { tuple, .. } => {
                if tuple.args != self.args.len() {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedArgumentCount {
                            meta: meta.clone(),
                            expected: tuple.args,
                            actual: self.args.len(),
                        },
                    ));
                }

                if tuple.args == 0 {
                    let tuple = path.span();
                    c.warnings
                        .remove_tuple_call_parens(c.source_id, span, tuple, c.context());
                }
            }
            CompileMetaKind::Function { .. } => (),
            CompileMetaKind::ConstFn { id, .. } => {
                let from = c.query.item_for(self)?;
                let const_fn = c.query.const_fn_for((self.span(), *id))?;

                if !needs.value() {
                    return Ok(Value::empty(span));
                }

                let value =
                    c.call_const_fn(self, &meta, &from, &*const_fn, self.args.as_slice())?;

                value.assemble_const(c, Needs::Value, self.span())?;
                return Ok(Value::unnamed(span, c));
            }
            _ => {
                return Err(CompileError::expected_meta(
                    span,
                    meta,
                    "something that can be called as a function",
                ));
            }
        };

        let mut values = Vec::with_capacity(self.args.len());

        for (expr, _) in &self.args {
            values.push(expr.assemble(c, Needs::Value)?);
        }

        for expr in values.into_iter().rev() {
            expr.pop(c)?;
        }

        let hash = Hash::type_hash(&meta.item.item);
        c.asm
            .push_with_comment(Inst::Call { hash, args }, span, meta.to_string());

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
