/// Header used during garbage collection.
pub(crate) struct Header(());

impl Header {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self(())
    }

    #[inline(always)]
    pub(crate) fn mark(&self) {}
}
