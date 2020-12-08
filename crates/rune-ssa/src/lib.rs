//! The state machine assembler of Rune.

mod block;
mod constant;
mod error;
mod global;
mod inst;
mod internal;
mod phi;
mod program;

pub use self::block::{Block, BlockJump};
pub use self::constant::Constant;
pub use self::error::Error;
pub use self::global::{BlockId, ConstId, ValueId};
pub use self::inst::Inst;
pub use self::phi::Phi;
pub use self::program::Program;

#[cfg(test)]
mod tests {
    use super::{Constant, Error, Program};

    #[test]
    fn test_basic_sm() -> Result<(), Error> {
        let mut sm = Program::new();

        let block = sm.named("main");
        let then = sm.block();

        let else_value = then.input();
        then.return_(else_value);

        // Define one input variable to the block.
        let a = block.input();
        let b = block.constant(Constant::Integer(42));
        let unit = block.constant(Constant::Unit);
        let c = block.add(a, b);

        let d = block.cmp_lt(c, b);
        block.jump_if(d, &then, &[b])?;
        block.return_(unit);

        println!("{}", sm.dump());
        Ok(())
    }
}
