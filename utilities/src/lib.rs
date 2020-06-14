#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(test, not(feature = "std")))]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
#[doc(hidden)]
pub use alloc::{boxed::Box, format, vec, vec::Vec};

#[cfg(feature = "std")]
#[allow(unused_imports)]
#[doc(hidden)]
pub use std::{boxed::Box, format, vec, vec::Vec};

pub mod biginteger;
pub mod bititerator;
#[macro_use]
pub mod bytes;
pub mod error;
pub mod rand;
pub mod variable_length_integer;

#[cfg(not(feature = "std"))]
pub mod io;

#[cfg(feature = "std")]
pub use std::io;

pub mod serialize;
pub use self::serialize::*;

#[cfg(not(feature = "std"))]
fn error(_msg: &'static str) -> io::Error {
    io::Error
}

#[cfg(feature = "std")]
fn error(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg)
}

#[macro_export]
macro_rules! unwrap_option_or_continue {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! unwrap_result_or_continue {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(_) => continue,
        }
    };
}

#[macro_export]
macro_rules! unwrap_option_or_error {
    ($e:expr; $err:expr) => {
        match $e {
            Some(val) => val,
            None => return Err($err),
        }
    };
}

/// Returns log2
pub fn log2(x: usize) -> u32 {
    if x <= 1 {
        return 0;
    }

    let n = x.leading_zeros();
    core::mem::size_of::<usize>() as u32 * 8 - n
}

/// Return ceil(x/y)
pub fn div_ceil(x: usize, y: usize) -> usize {
    (x + y - 1) / y
}
