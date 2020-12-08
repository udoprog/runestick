use crate::BlockId;
use thiserror::Error;

/// Error raised during machine construction.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("block {block} is finalized, and cannot be flowed into")]
    BlockControlFinalized { block: BlockId },
    #[error("mismatch in block inputs ({block}), expected {expected} but got {actual}")]
    BlockInputMismatch {
        block: BlockId,
        expected: usize,
        actual: usize,
    },
}
