//! The state machine assembler of Rune.

mod block;
mod constant;
mod error;
mod global;
mod inst;
mod internal;
mod phi;
mod program;
mod term;

pub use self::block::Block;
pub use self::constant::Constant;
pub use self::error::Error;
pub use self::global::{BlockId, ConstId, Var};
pub use self::inst::Inst;
pub use self::phi::{Dep, Phi};
pub use self::program::Program;
pub use self::term::Term;

#[cfg(test)]
mod tests {
    use super::{Constant, Error, Program};

    #[test]
    fn test_basic_program() -> Result<(), Error> {
        let mut program = Program::new();

        let end = program.block();
        let entry = program.named("main");
        let then_block = program.block();

        let a = entry.input();
        let b = entry.constant(Constant::Integer(10));
        let condition = entry.cmp_lt(a, b);
        entry.jump_if(condition, &then_block, &end);

        let c = then_block.constant(Constant::Integer(1));
        then_block.assign_add(a, a, c);
        then_block.jump(&end);

        end.return_(a);

        println!("{}", program.dump());
        Ok(())
    }
}
