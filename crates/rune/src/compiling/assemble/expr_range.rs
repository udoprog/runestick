use crate::compiling::assemble::prelude::*;

/// Compile a range expression.
impl Assemble for ast::ExprRange {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Value> {
        let span = self.span();
        log::trace!("ExprRange => {:?}", c.source.source(span));

        let guard = c.scopes.push();

        if needs.value() {
            let from = if let Some(from) = &self.from {
                from.assemble(c, needs)?.pop(c)?;
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    from.span(),
                );
                Value::unnamed(from.span(), c)
            } else {
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                );
                Value::unnamed(span, c)
            };

            let to = if let Some(to) = &self.to {
                to.assemble(c, needs)?.pop(c)?;
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    to.span(),
                );
                Value::unnamed(to.span(), c)
            } else {
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                );
                Value::unnamed(span, c)
            };

            let limits = match &self.limits {
                ast::ExprRangeLimits::HalfOpen(..) => InstRangeLimits::HalfOpen,
                ast::ExprRangeLimits::Closed(..) => InstRangeLimits::Closed,
            };

            to.pop(c)?;
            from.pop(c)?;

            c.asm.push(Inst::Range { limits }, span);
        } else {
            if let Some(from) = &self.from {
                from.assemble(c, needs)?;
            }

            if let Some(to) = &self.to {
                to.assemble(c, needs)?;
            }
        }

        guard.pop(span, c)?;

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
            return Ok(Value::empty(span));
        }

        Ok(Value::unnamed(span, c))
    }
}
