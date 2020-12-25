use crate::global::Global;
use crate::internal::commas;
use crate::{BlockId, Constant, Error, Inst, Phi, ValueId};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// A jump into a block which carries a collection of values with it as inputs
/// to the block.
#[derive(Debug, Clone)]
pub struct BlockJump(BlockId, Box<[ValueId]>);

impl fmt::Display for BlockJump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.1.is_empty() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{}({})", self.0, commas(self.1.as_ref()))
        }
    }
}

/// Macro to help build a binary op.
macro_rules! block_binary_op {
    ($name:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(&self, lhs: ValueId, rhs: ValueId) -> ValueId {
            let value = self.inner.global.value();
            self.inner
                .assignments
                .borrow_mut()
                .insert(value, Inst::$variant(lhs, rhs));
            value
        }
    }
}

/// A block containing a sequence of assignments.
///
/// A block carries a definition of its entry.
/// The entry is the sequence of input variables the block expects.
#[derive(Clone)]
pub struct Block {
    inner: Rc<BlockInner>,
}

impl Block {
    /// Construct a new empty block.
    pub(crate) fn new(global: Global, name: Option<Box<str>>) -> Self {
        let id = global.block();

        Self {
            inner: Rc::new(BlockInner {
                id,
                name,
                global,
                finalized: Cell::new(false),
                inputs: RefCell::new(Vec::new()),
                assignments: RefCell::new(BTreeMap::new()),
                instructions: RefCell::new(Vec::new()),
                ancestors: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Finalize the block.
    pub fn finalize(&self) {
        self.inner.finalized.set(true);
    }

    /// Get the identifier of the block.
    #[inline]
    pub fn id(&self) -> BlockId {
        self.inner.id
    }

    /// Perform a diagnostical dump of a block.
    #[inline]
    pub fn dump(&self) -> BlockDump<'_> {
        BlockDump(self)
    }

    /// Allocate an input variable.
    pub fn input(&self) -> ValueId {
        let value = self.inner.global.value();
        self.inner.inputs.borrow_mut().push(value);
        self.inner
            .assignments
            .borrow_mut()
            .insert(value, Inst::Phi(Phi::default()));
        value
    }

    /// Define a unit.
    pub fn unit(&self) -> ValueId {
        self.constant(Constant::Unit)
    }

    /// Load a constant as a variable.
    pub fn constant(&self, constant: Constant) -> ValueId {
        let value = self.inner.global.value();
        let const_id = self.inner.global.constant(constant);
        self.inner
            .assignments
            .borrow_mut()
            .insert(value, Inst::Const(const_id));
        value
    }

    /// Force a use of the given value.
    pub fn use_(&self, value: ValueId) {
        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::Value(value));
    }

    /// Unconditionally jump to the given block.
    pub fn jump(&self, block: &Block, input: &[ValueId]) -> Result<(), Error> {
        let jump = self.block_jump(block, input)?;

        self.inner.instructions.borrow_mut().push(Inst::Jump(jump));

        Ok(())
    }

    /// Perform a conditional jump to the given block with the specified inputs
    /// if the given condition is true.
    pub fn jump_if(&self, cond: ValueId, block: &Block, input: &[ValueId]) -> Result<(), Error> {
        let jump = self.block_jump(block, input)?;

        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::JumpIf(cond, jump));

        Ok(())
    }

    block_binary_op!(add, Add, "Compute `lhs + rhs`.");
    block_binary_op!(sub, Sub, "Compute `lhs - rhs`.");
    block_binary_op!(div, Div, "Compute `lhs / rhs`.");
    block_binary_op!(mul, Mul, "Compute `lhs * rhs`.");
    block_binary_op!(cmp_lt, CmpLt, "Compare if `lhs < rhs`.");
    block_binary_op!(cmp_lte, CmpLte, "Compare if `lhs <= rhs`.");
    block_binary_op!(cmp_eq, CmpEq, "Compare if `lhs == rhs`.");
    block_binary_op!(cmp_gt, CmpGt, "Compare if `lhs > rhs`.");
    block_binary_op!(cmp_gte, CmpGte, "Compare if `lhs >= rhs`.");

    /// Unconditionally return from this the procedure this block belongs to.
    pub fn return_unit(&self) {
        let value = self.unit();

        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::Return(value));
    }

    /// Unconditionally return from this the procedure this block belongs to.
    pub fn return_(&self, value: ValueId) {
        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::Return(value));
    }

    /// Construct and validate a block jump.
    fn block_jump(&self, block: &Block, input: &[ValueId]) -> Result<BlockJump, Error> {
        if block.inner.finalized.get() {
            return Err(Error::BlockControlFinalized { block: block.id() });
        }

        let inputs = block.inner.inputs.borrow();

        if inputs.len() != input.len() {
            return Err(Error::BlockInputMismatch {
                block: block.id(),
                expected: inputs.len(),
                actual: input.len(),
            });
        }

        for (assignment, input) in inputs.iter().zip(input) {
            let mut inst = block.inner.assignments.borrow_mut();

            if let Some(Inst::Phi(phi)) = inst.get_mut(assignment) {
                phi.push(*input);
            }
        }

        // Mark this block as an ancestor to the block we're jumping to.
        block.inner.ancestors.borrow_mut().push(self.inner.id);
        Ok(BlockJump(block.inner.id, input.into()))
    }
}

pub struct BlockDump<'a>(&'a Block);

impl fmt::Display for BlockDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inputs = self.0.inner.inputs.borrow();
        let ancestors = self.0.inner.ancestors.borrow();

        if inputs.len() == 0 {
            write!(f, "{}", self.0.id())?;
        } else {
            write!(f, "{}({})", self.0.id(), commas(&*inputs))?;
        }

        if ancestors.is_empty() {
            write!(f, ":")?;
        } else {
            write!(f, ": {}", commas(&ancestors[..]))?;
        }

        if let Some(name) = &self.0.inner.name {
            write!(f, " // {}", name)?;
        }

        writeln!(f)?;

        for (v, inst) in self.0.inner.assignments.borrow().iter() {
            writeln!(f, "  {} <- {}", v, inst.dump())?;
        }

        for inst in self.0.inner.instructions.borrow().iter() {
            writeln!(f, "  {}", inst.dump())?;
        }

        Ok(())
    }
}

struct BlockInner {
    /// The identifier of the block.
    id: BlockId,
    /// If the block is finalized or not.
    ///
    /// Control flows can only be added to non-finalized blocks.
    finalized: Cell<bool>,
    /// The (optional) name of the block for debugging and symbolic purposes.
    name: Option<Box<str>>,
    /// Global shared stack machine state.
    global: Global,
    /// Input variables.
    inputs: RefCell<Vec<ValueId>>,
    /// Instructions being built.
    assignments: RefCell<BTreeMap<ValueId, Inst>>,
    /// Instructions that do not produce a value.
    instructions: RefCell<Vec<Inst>>,
    /// Ancestor blocks.
    ancestors: RefCell<Vec<BlockId>>,
}
