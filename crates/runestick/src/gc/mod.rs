//! Optional garbage collector for Runestick
//!
//! The collector itself is accessible and operaters through thread-local
//! interfaces.

#[cfg(feature = "gc")]
#[path = "real.rs"]
mod imp;

#[cfg(not(feature = "gc"))]
#[path = "mock.rs"]
mod imp;

pub(crate) use self::imp::Header;

/// Trait for types which can be marked.
pub trait Mark {
    /// Perform a mark over the given type.
    fn mark(&self);
}

impl<T> Mark for Option<T>
where
    T: Mark,
{
    fn mark(&self) {
        match self {
            Some(some) => some.mark(),
            None => (),
        }
    }
}

impl<T, E> Mark for Result<T, E>
where
    T: Mark,
    E: Mark,
{
    fn mark(&self) {
        match self {
            Ok(ok) => ok.mark(),
            Err(err) => err.mark(),
        }
    }
}

impl<T> Mark for Vec<T>
where
    T: Mark,
{
    fn mark(&self) {
        for m in self {
            m.mark();
        }
    }
}

impl<T> Mark for Box<[T]>
where
    T: Mark,
{
    fn mark(&self) {
        for m in self.iter() {
            m.mark();
        }
    }
}

impl Mark for String {
    fn mark(&self) {}
}
