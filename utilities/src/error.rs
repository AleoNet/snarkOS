#[cfg(feature = "std")]
pub use std::error::Error;

#[cfg(not(feature = "std"))]
pub trait Error: core::fmt::Debug + core::fmt::Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[cfg(not(feature = "std"))]
impl<'a, E: Error + 'a> From<E> for crate::Box<dyn Error + 'a> {
    fn from(err: E) -> crate::Box<dyn Error + 'a> {
        crate::Box::new(err)
    }
}

#[cfg(not(feature = "std"))]
impl<T: Error> Error for crate::Box<T> {}

#[cfg(not(feature = "std"))]
impl Error for crate::String {}

#[cfg(not(feature = "std"))]
impl Error for crate::io::Error {}
