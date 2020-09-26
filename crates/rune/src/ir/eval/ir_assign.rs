use crate::ir::eval::prelude::*;

impl Eval<&ir::IrAssign> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_assign: &ir::IrAssign, used: Used) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(ir_assign)?;
        let value = self.eval(&*ir_assign.value, used)?;

        self.scopes.mut_target(&ir_assign.target, move |t| {
            ir_assign.op.assign(ir_assign, t, value)
        })?;

        Ok(IrValue::Unit)
    }
}