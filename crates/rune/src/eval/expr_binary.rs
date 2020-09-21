use crate::eval::prelude::*;

impl Eval<&ast::ExprBinary> for ConstCompiler<'_> {
    fn eval(&mut self, binary: &ast::ExprBinary, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(binary.span())?;

        if binary.op.is_assign() {
            return op_assign(self, binary, used);
        }

        let lhs = self.eval(&*binary.lhs, used)?;
        let rhs = self.eval(&*binary.rhs, used)?;

        let span = binary.lhs.span().join(binary.rhs.span());

        match (lhs, rhs) {
            (ConstValue::Integer(a), ConstValue::Integer(b)) => {
                match binary.op {
                    ast::BinOp::Add => {
                        return Ok(checked_int(
                            a,
                            b,
                            i64::checked_add,
                            "integer overflow",
                            span,
                        )?);
                    }
                    ast::BinOp::Sub => {
                        return Ok(checked_int(
                            a,
                            b,
                            i64::checked_sub,
                            "integer underflow",
                            span,
                        )?);
                    }
                    ast::BinOp::Mul => {
                        return Ok(checked_int(
                            a,
                            b,
                            i64::checked_mul,
                            "integer overflow",
                            span,
                        )?);
                    }
                    ast::BinOp::Div => {
                        return Ok(checked_int(
                            a,
                            b,
                            i64::checked_div,
                            "integer division by zero",
                            span,
                        )?);
                    }
                    ast::BinOp::Shl => {
                        let b = u32::try_from(b).map_err(|_| {
                            CompileError::const_error(
                                &binary.rhs,
                                "cannot be converted to shift operand",
                            )
                        })?;

                        let n = a.checked_shl(b).ok_or_else(|| {
                            CompileError::const_error(span, "integer shift overflow")
                        })?;

                        return Ok(ConstValue::Integer(n));
                    }
                    ast::BinOp::Shr => {
                        let b = u32::try_from(b).map_err(|_| {
                            CompileError::const_error(
                                &binary.rhs,
                                "cannot be converted to shift operand",
                            )
                        })?;

                        let n = a.checked_shr(b).ok_or_else(|| {
                            CompileError::const_error(span, "integer shift underflow")
                        })?;

                        return Ok(ConstValue::Integer(n));
                    }
                    ast::BinOp::Lt => return Ok(ConstValue::Bool(a < b)),
                    ast::BinOp::Lte => return Ok(ConstValue::Bool(a <= b)),
                    ast::BinOp::Eq => return Ok(ConstValue::Bool(a == b)),
                    ast::BinOp::Gt => return Ok(ConstValue::Bool(a > b)),
                    ast::BinOp::Gte => return Ok(ConstValue::Bool(a >= b)),
                    _ => (),
                };
            }
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                match binary.op {
                    ast::BinOp::Add => return Ok(ConstValue::Float(a + b)),
                    ast::BinOp::Sub => return Ok(ConstValue::Float(a - b)),
                    ast::BinOp::Mul => return Ok(ConstValue::Float(a * b)),
                    ast::BinOp::Div => return Ok(ConstValue::Float(a / b)),
                    ast::BinOp::Lt => return Ok(ConstValue::Bool(a < b)),
                    ast::BinOp::Lte => return Ok(ConstValue::Bool(a <= b)),
                    ast::BinOp::Eq => return Ok(ConstValue::Bool(a == b)),
                    ast::BinOp::Gt => return Ok(ConstValue::Bool(a > b)),
                    ast::BinOp::Gte => return Ok(ConstValue::Bool(a >= b)),
                    _ => (),
                };
            }
            _ => (),
        }

        Err(EvalOutcome::not_const(binary))
    }
}

fn checked_int(
    a: i64,
    b: i64,
    op: impl FnOnce(i64, i64) -> Option<i64>,
    msg: &'static str,
    span: Span,
) -> Result<ConstValue, CompileError> {
    let n = op(a, b).ok_or_else(|| CompileError::const_error(span, msg))?;
    Ok(ConstValue::Integer(n))
}

fn op_assign(
    this: &mut ConstCompiler<'_>,
    binary: &ast::ExprBinary,
    used: Used,
) -> Result<ConstValue, EvalOutcome> {
    match binary.op {
        ast::BinOp::Assign => match &*binary.lhs {
            ast::Expr::Path(path) => {
                if let Some(name) = path.try_as_ident() {
                    let name = this.resolve(name)?;

                    let value = this.eval(&*binary.rhs, used)?;

                    this.scopes.replace(name.as_ref(), value, binary.span())?;
                    return Ok(ConstValue::Unit);
                }
            }
            _ => (),
        },
        _ => (),
    }

    Err(EvalOutcome::not_const(binary))
}
