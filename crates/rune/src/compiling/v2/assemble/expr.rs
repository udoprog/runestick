use crate::compiling::v2::assemble::prelude::*;

/// Assembler for a block.
impl Assemble for ast::Expr {
    fn assemble(&self, block: &Block, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.source.source(span));

        match self {
            ast::Expr::Lit(expr) => expr.assemble(block, c),
            ast::Expr::Binary(expr) => expr.assemble(block, c),
            ast::Expr::Path(expr) => expr.assemble(block, c),
            _ => Err(CompileError::msg(span, "unsupported expr")),
        }
    }
}

impl Assemble for ast::ExprLit {
    fn assemble(&self, block: &Block, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        use num::ToPrimitive as _;

        let span = self.span();
        log::trace!("ExprLit => {:?}", c.source.source(span));

        Ok(match &self.lit {
            ast::Lit::Bool(b) => block.constant(Constant::Bool(b.value)),
            ast::Lit::Byte(b) => {
                let b = c.resolve(b)?;
                block.constant(Constant::Byte(b))
            }
            ast::Lit::Str(s) => {
                let s = c.resolve(s)?;
                block.constant(Constant::String(s.into()))
            }
            ast::Lit::ByteStr(b) => {
                let b = c.resolve(b)?;
                block.constant(Constant::Bytes(b.into()))
            }
            ast::Lit::Char(ch) => {
                let ch = c.resolve(ch)?;
                block.constant(Constant::Char(ch))
            }
            ast::Lit::Number(n) => match c.resolve(n)? {
                ast::Number::Float(n) => block.constant(Constant::Float(n)),
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

                    block.constant(Constant::Integer(n))
                }
            },
        })
    }
}

impl Assemble for ast::ExprBinary {
    fn assemble(&self, block: &Block, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", c.source.source(span));

        let lhs = self.lhs.assemble(block, c)?;
        let rhs = self.rhs.assemble(block, c)?;

        match self.op {
            ast::BinOp::Add => Ok(block.add(lhs, rhs)),
            ast::BinOp::Sub => Ok(block.sub(lhs, rhs)),
            ast::BinOp::Div => Ok(block.div(lhs, rhs)),
            ast::BinOp::Mul => Ok(block.mul(lhs, rhs)),
            _ => return Err(CompileError::msg(self.op_span(), "unsupported op")),
        }
    }
}

impl Assemble for ast::Path {
    fn assemble(&self, _: &Block, c: &mut Compiler<'_>) -> CompileResult<ValueId> {
        let span = self.span();
        log::trace!("Path => {:?}", c.source.source(span));

        if let Some(ident) = self.try_as_ident() {
            let name = c.resolve(ident)?;
            return Ok(c.scope.get(span, &name)?);
        }

        Err(CompileError::msg(span, "unsupported path"))
    }
}
