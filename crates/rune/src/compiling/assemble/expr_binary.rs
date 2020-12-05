use crate::compiling::assemble::prelude::*;

/// Compile a binary expression.
impl Assemble for ast::ExprBinary {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", c.source.source(span));
        log::trace!(
            "ExprBinary {{ lhs => {:?} }}",
            c.source.source(self.lhs.span())
        );
        log::trace!("ExprBinary {{ op => {:?} }}", self.op);
        log::trace!(
            "ExprBinary {{ rhs => {:?} }}",
            c.source.source(self.rhs.span())
        );

        // Special expressions which operates on the stack in special ways.
        if self.op.is_assign() {
            let value = compile_assign_binop(c, &self.lhs, &self.rhs, self.op, needs)?;
            return Ok(value);
        }

        if self.op.is_conditional() {
            let value = compile_conditional_binop(c, &self.lhs, &self.rhs, self.op, needs)?;
            return Ok(value);
        }

        // NB: need to declare these as anonymous local variables so that they
        // get cleaned up in case there is an early break (return, try, ...).
        let a = self.lhs.assemble(c, Needs::Value)?;
        let b = self.rhs.assemble(c, rhs_needs_of(self.op))?;

        let b = b.consume_into_address(c)?;
        let a = a.consume_into_address(c)?;

        let op = match self.op {
            ast::BinOp::Eq => InstOp::Eq,
            ast::BinOp::Neq => InstOp::Neq,
            ast::BinOp::Lt => InstOp::Lt,
            ast::BinOp::Gt => InstOp::Gt,
            ast::BinOp::Lte => InstOp::Lte,
            ast::BinOp::Gte => InstOp::Gte,
            ast::BinOp::Is => InstOp::Is,
            ast::BinOp::IsNot => InstOp::IsNot,
            ast::BinOp::And => InstOp::And,
            ast::BinOp::Or => InstOp::Or,
            ast::BinOp::Add => InstOp::Add,
            ast::BinOp::Sub => InstOp::Sub,
            ast::BinOp::Div => InstOp::Div,
            ast::BinOp::Mul => InstOp::Mul,
            ast::BinOp::Rem => InstOp::Rem,
            ast::BinOp::BitAnd => InstOp::BitAnd,
            ast::BinOp::BitXor => InstOp::BitXor,
            ast::BinOp::BitOr => InstOp::BitOr,
            ast::BinOp::Shl => InstOp::Shl,
            ast::BinOp::Shr => InstOp::Shr,

            op => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryOp { op },
                ));
            }
        };

        c.asm.push(Inst::Op { op, a, b }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}

/// Get the need of the right-hand side operator from the type of the
/// operator.
fn rhs_needs_of(op: ast::BinOp) -> Needs {
    match op {
        ast::BinOp::Is | ast::BinOp::IsNot => Needs::Type,
        _ => Needs::Value,
    }
}

fn compile_conditional_binop(
    c: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<Value> {
    let span = lhs.span().join(rhs.span());

    let end_label = c.asm.new_label("conditional_end");
    lhs.assemble(c, Needs::Value)?;

    match bin_op {
        ast::BinOp::And => {
            c.asm.jump_if_not_or_pop(end_label, lhs.span());
        }
        ast::BinOp::Or => {
            c.asm.jump_if_or_pop(end_label, lhs.span());
        }
        op => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryOp { op },
            ));
        }
    }

    rhs.assemble(c, Needs::Value)?;

    c.asm.label(end_label)?;

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
        return Ok(Value::empty(span));
    }

    Ok(Value::unnamed(span, c))
}

fn compile_assign_binop(
    c: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<Value> {
    let span = lhs.span().join(rhs.span());

    let supported = match lhs {
        // <var> <op> <expr>
        ast::Expr::Path(path) if path.rest.is_empty() => {
            rhs.assemble(c, Needs::Value)?.pop(c)?;

            let segment = path
                .first
                .try_as_ident()
                .ok_or_else(|| CompileError::msg(path, "unsupported path segment"))?;
            let ident = segment.resolve(c.storage, &*c.source)?;
            let var = c.scopes.get_var(&*ident, c.source_id, c.visitor, span)?;

            Some(InstTarget::Offset(var.offset))
        }
        // <expr>.<field> <op> <value>
        ast::Expr::FieldAccess(field_access) => {
            let expr = field_access.expr.assemble(c, Needs::Value)?;
            let rhs = rhs.assemble(c, Needs::Value)?;

            rhs.pop(c)?;
            expr.pop(c)?;

            // field assignment
            match &field_access.expr_field {
                ast::ExprField::Ident(index) => {
                    let n = index.resolve(c.storage, &*c.source)?;
                    let n = c.unit.new_static_string(index, n.as_ref())?;

                    Some(InstTarget::Field(n))
                }
                ast::ExprField::LitNumber(field) => {
                    let span = field.span();

                    let number = field.resolve(c.storage, &*c.source)?;
                    let index = number.as_tuple_index().ok_or_else(|| {
                        CompileError::new(span, CompileErrorKind::UnsupportedTupleIndex { number })
                    })?;

                    Some(InstTarget::TupleField(index))
                }
            }
        }
        _ => None,
    };

    let target = match supported {
        Some(target) => target,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryExpr,
            ));
        }
    };

    let op = match bin_op {
        ast::BinOp::AddAssign => InstAssignOp::Add,
        ast::BinOp::SubAssign => InstAssignOp::Sub,
        ast::BinOp::MulAssign => InstAssignOp::Mul,
        ast::BinOp::DivAssign => InstAssignOp::Div,
        ast::BinOp::RemAssign => InstAssignOp::Rem,
        ast::BinOp::BitAndAssign => InstAssignOp::BitAnd,
        ast::BinOp::BitXorAssign => InstAssignOp::BitXor,
        ast::BinOp::BitOrAssign => InstAssignOp::BitOr,
        ast::BinOp::ShlAssign => InstAssignOp::Shl,
        ast::BinOp::ShrAssign => InstAssignOp::Shr,
        _ => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryExpr,
            ));
        }
    };

    c.asm.push(Inst::Assign { target, op }, span);

    if !needs.value() {
        return Ok(Value::empty(span));
    }

    c.asm.push(Inst::unit(), span);
    Ok(Value::unnamed(span, c))
}
