use crate::compiling::v2::assemble::prelude::*;

/// Assembler for a block.
impl Assemble for ast::Expr {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.source.source(span));

        match self {
            ast::Expr::Lit(expr) => expr.assemble(c),
            ast::Expr::Binary(expr) => expr.assemble(c),
            ast::Expr::Path(expr) => expr.assemble(c),
            ast::Expr::If(expr) => expr.assemble(c),
            _ => Err(CompileError::msg(span, "unsupported expr")),
        }
    }
}

impl Assemble for ast::ExprLit {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        use num::ToPrimitive as _;

        let span = self.span();
        log::trace!("ExprLit => {:?}", c.source.source(span));

        Ok(match &self.lit {
            ast::Lit::Bool(b) => c.block.constant(Constant::Bool(b.value)),
            ast::Lit::Byte(b) => {
                let b = c.resolve(b)?;
                c.block.constant(Constant::Byte(b))
            }
            ast::Lit::Str(s) => {
                let s = c.resolve(s)?;
                c.block.constant(Constant::String(s.into()))
            }
            ast::Lit::ByteStr(b) => {
                let b = c.resolve(b)?;
                c.block.constant(Constant::Bytes(b.into()))
            }
            ast::Lit::Char(ch) => {
                let ch = c.resolve(ch)?;
                c.block.constant(Constant::Char(ch))
            }
            ast::Lit::Number(n) => match c.resolve(n)? {
                ast::Number::Float(n) => c.block.constant(Constant::Float(n)),
                ast::Number::Integer(n) => {
                    let n = match n.to_i64() {
                        Some(n) => n,
                        None => {
                            return Err(CompileError::new(
                                span,
                                ParseErrorKind::BadNumberOutOfBounds,
                            ));
                        }
                    };

                    c.block.constant(Constant::Integer(n))
                }
            },
        })
    }
}

impl Assemble for ast::ExprBinary {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", c.source.source(span));

        let lhs = self.lhs.assemble(c)?;
        let rhs = self.rhs.assemble(c)?;

        match self.op {
            ast::BinOp::Add => Ok(c.block.add(lhs, rhs)),
            ast::BinOp::Sub => Ok(c.block.sub(lhs, rhs)),
            ast::BinOp::Div => Ok(c.block.div(lhs, rhs)),
            ast::BinOp::Mul => Ok(c.block.mul(lhs, rhs)),
            _ => return Err(CompileError::msg(self.op_span(), "unsupported op")),
        }
    }
}

impl Assemble for ast::Path {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Path => {:?}", c.source.source(span));

        if let Some(ident) = self.try_as_ident() {
            let name = c.resolve(ident)?;
            return Ok(c.scope.get(span, &name)?);
        }

        Err(CompileError::msg(span, "unsupported path"))
    }
}

impl Assemble for ast::ExprIf {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("ExprIf => {:?}", c.source.source(span));

        // assemble the output block of the expression.
        let out = c.program.block();
        let value = out.input();

        let then = self.block.assemble_block(c)?;

        let cond = self.condition.assemble(c)?;

        c.block.jump_if(cond, &then, &[]).with_span(span)?;

        let unit = c.block.constant(Constant::Unit);

        c.block.jump(&out, &[unit]).with_span(span)?;
        c.block = out;
        Ok(value)
    }
}

impl Assemble for ast::Condition {
    fn assemble(&self, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Condition => {:?}", c.source.source(span));

        match self {
            ast::Condition::Expr(expr) => expr.assemble(c),
            ast::Condition::ExprLet(_) => Err(CompileError::msg(span, "unsupported condition")),
        }
    }
}
