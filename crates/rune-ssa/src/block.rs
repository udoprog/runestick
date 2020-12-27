use crate::global::Global;
use crate::internal::commas;
use crate::{BlockId, Constant, Dep, Error, Inst, Phi, Term, Var};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// Macro to help build a binary op.
macro_rules! block_binary_op {
    ($new:ident, $assign:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $new(&self, a: Var, b: Var) -> Var {
            self.read(a);
            self.read(b);
            self.assign_new(Inst::$variant(a, b))
        }

        #[doc = $doc]
        pub fn $assign(&self, id: Var, a: Var, b: Var) {
            self.read(a);
            self.read(b);
            self.assign(id, Inst::$variant(a, b));
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
    pub(crate) fn new(id: BlockId, global: Global, name: Option<Box<str>>) -> Self {
        Self {
            inner: Rc::new(BlockInner {
                id,
                inputs: Cell::new(0),
                name,
                global,
                finalized: Cell::new(false),
                assignments: RefCell::new(BTreeMap::new()),
                term: RefCell::new(Term::Panic),
                ancestors: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Read the given variable, looking it up recursively in ancestor blocks
    /// and memoizing as needed.
    fn read(&self, var: Var) {
        // Local assignment that is already present.
        if self.inner.assignments.borrow().contains_key(&var) {
            return;
        }

        self.read_recursive(var);
    }

    /// Read the given variable recursively.
    fn read_recursive(&self, var: Var) -> Dep {
        let dep = Dep {
            block: self.id(),
            var,
        };

        if let Some(inst) = self.inner.assignments.borrow().get(&var) {
            return match inst {
                Inst::Var(dep) => dep.clone(),
                _ => dep,
            };
        }

        self.inner
            .assignments
            .borrow_mut()
            .insert(var, Inst::Phi(Phi::default()));

        let mut dependencies = Vec::new();

        for ancestor in self.inner.ancestors.borrow().iter() {
            let block = self.inner.global.get_block(*ancestor);
            let dep = block.read_recursive(var);
            dependencies.push(dep);
        }

        if let Some(Inst::Phi(phi)) = self.inner.assignments.borrow_mut().get_mut(&var) {
            for d in dependencies {
                phi.insert(d);
            }
        }

        dep
    }

    /// Assign an instruction to a new vvar.
    fn assign_new(&self, inst: Inst) -> Var {
        let var = self.inner.global.var();
        self.assign(var, inst);
        var
    }

    /// Assign an instruction to an existing var.
    fn assign(&self, id: Var, inst: Inst) {
        self.inner.assignments.borrow_mut().insert(id, inst);
    }

    /// Define an input into the block.
    pub fn input(&self) -> Var {
        let id = self.inner.global.var();
        let input = self.inner.inputs.get();
        self.inner.inputs.set(input + 1);
        self.assign(id, Inst::Input(input));
        id
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

    /// Define a unit.
    pub fn unit(&self) -> Var {
        self.constant(Constant::Unit)
    }

    /// Assign a unit.
    pub fn assign_unit(&self, id: Var) {
        self.assign_constant(id, Constant::Unit);
    }

    /// Define a constant.
    pub fn constant(&self, constant: Constant) -> Var {
        let const_id = self.inner.global.constant(constant);
        self.assign_new(Inst::Const(const_id))
    }

    /// Assign a constant.
    pub fn assign_constant(&self, id: Var, constant: Constant) {
        let const_id = self.inner.global.constant(constant);
        self.assign(id, Inst::Const(const_id));
    }

    block_binary_op!(add, assign_add, Add, "Compute `lhs + rhs`.");
    block_binary_op!(sub, assign_sub, Sub, "Compute `lhs - rhs`.");
    block_binary_op!(div, assign_div, Div, "Compute `lhs / rhs`.");
    block_binary_op!(mul, assign_mul, Mul, "Compute `lhs * rhs`.");
    block_binary_op!(cmp_lt, assign_cmp_lt, CmpLt, "Compare if `lhs < rhs`.");
    block_binary_op!(cmp_lte, assign_cmp_lte, CmpLte, "Compare if `lhs <= rhs`.");
    block_binary_op!(cmp_eq, assign_cmp_eq, CmpEq, "Compare if `lhs == rhs`.");
    block_binary_op!(cmp_gt, assign_cmp_gt, CmpGt, "Compare if `lhs > rhs`.");
    block_binary_op!(cmp_gte, assign_cmp_gte, CmpGte, "Compare if `lhs >= rhs`.");

    /// Perform an unconditional jump to the given block with the specified
    /// inputs.
    pub fn jump(&self, block: &Block) {
        block.inner.ancestors.borrow_mut().push(self.id());

        *self.inner.term.borrow_mut() = Term::Jump { block: block.id() };
    }

    /// Perform a conditional jump to the given block with the specified inputs
    /// if the given condition is true.
    pub fn jump_if(&self, condition: Var, then_block: &Block, else_block: &Block) {
        self.read(condition);

        then_block.inner.ancestors.borrow_mut().push(self.id());
        else_block.inner.ancestors.borrow_mut().push(self.id());

        *self.inner.term.borrow_mut() = Term::JumpIf {
            condition,
            then_block: then_block.id(),
            else_block: else_block.id(),
        };
    }

    /// Return from this the procedure this block belongs to.
    pub fn return_unit(&self) {
        let var = self.unit();
        *self.inner.term.borrow_mut() = Term::Return { var };
    }

    /// Return from this the procedure this block belongs to.
    pub fn return_(&self, var: Var) {
        self.read(var);
        *self.inner.term.borrow_mut() = Term::Return { var };
    }
}

pub struct BlockDump<'a>(&'a Block);

impl fmt::Display for BlockDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ancestors = self.0.inner.ancestors.borrow();

        write!(f, "{}", self.0.id())?;

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

        writeln!(f, "  {}", self.0.inner.term.borrow().dump())?;
        Ok(())
    }
}

struct BlockInner {
    /// The identifier of the block.
    id: BlockId,
    /// The number of inputs in the block.
    inputs: Cell<usize>,
    /// If the block is finalized or not.
    ///
    /// Control flows can only be added to non-finalized blocks.
    finalized: Cell<bool>,
    /// The (optional) name of the block for debugging and symbolic purposes.
    name: Option<Box<str>>,
    /// Global shared stack machine state.
    global: Global,
    /// Instructions being built.
    assignments: RefCell<BTreeMap<Var, Inst>>,
    /// Instructions that do not produce a value.
    term: RefCell<Term>,
    /// Ancestor blocks.
    ancestors: RefCell<Vec<BlockId>>,
}
