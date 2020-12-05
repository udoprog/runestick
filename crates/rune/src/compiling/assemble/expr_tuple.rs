use crate::compiling::assemble::prelude::*;

macro_rules! tuple {
    ($slf:expr, $variant:ident, $c:ident, $span:expr, $($var:ident),*) => {{
        let mut it = $slf.items.iter();

        $(
        let ($var, _) = it.next().ok_or_else(|| CompileError::new($span, CompileErrorKind::Custom { message: "items ended unexpectedly" }))?;
        let $var = $var.assemble($c, Needs::Value)?;
        let $var = $var.consume_into_address($c)?;
        )*

        $c.asm.push(
            Inst::$variant {
                args: [$($var,)*],
            },
            $span,
        );
    }};
}

/// Compile a literal tuple.
impl Assemble for ast::ExprTuple {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprTuple => {:?}", c.source.source(span));

        if self.items.is_empty() {
            c.asm.push(Inst::unit(), span);
        } else {
            match self.items.len() {
                1 => tuple!(self, Tuple1, c, span, e1),
                2 => tuple!(self, Tuple2, c, span, e1, e2),
                3 => tuple!(self, Tuple3, c, span, e1, e2, e3),
                4 => tuple!(self, Tuple4, c, span, e1, e2, e3, e4),
                _ => {
                    let mut items = Vec::new();

                    for (expr, _) in &self.items {
                        items.push(expr.assemble(c, Needs::Value)?);
                    }

                    for item in items.into_iter().rev() {
                        item.pop(c)?;
                    }

                    c.asm.push(
                        Inst::Tuple {
                            count: self.items.len(),
                        },
                        span,
                    );
                }
            }
        }

        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
