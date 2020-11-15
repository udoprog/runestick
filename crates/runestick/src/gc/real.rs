use std::cell::Cell;

thread_local! {
    static GC: *const Gc = std::ptr::null();
}

/// Header used during garbage collection.
pub(crate) struct Header {
    marked: Cell<bool>,
}

impl Header {
    /// Construct a new default header.
    pub(crate) const fn new() -> Self {
        Self {
            marked: Cell::new(false),
        }
    }

    /// Mark the given header as reachable.
    pub(crate) fn mark(&self) {
        self.marked.set(true);
    }
}

/// thread-local garbage collector.
struct Gc {}
