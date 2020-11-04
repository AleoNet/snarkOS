// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

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
pub use biginteger::*;

pub mod bititerator;
pub use bititerator::*;

#[macro_use]
pub mod bytes;
pub use bytes::*;

pub mod error;
pub use error::*;

pub mod iterator;
pub use iterator::*;

pub mod math;
pub use math::*;

pub mod rand;
pub use self::rand::*;

pub mod serialize;
pub use serialize::*;

pub mod variable_length_integer;
pub use variable_length_integer::*;

#[cfg(not(feature = "std"))]
pub mod io;

#[cfg(feature = "std")]
pub use std::io;

#[cfg(not(feature = "std"))]
pub fn error(_msg: &'static str) -> io::Error {
    io::Error
}

#[cfg(feature = "std")]
pub fn error(msg: &'static str) -> io::Error {
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

use std::sync::atomic::AtomicBool;
pub static PROCESSING_SNARK_PARAMS: AtomicBool = AtomicBool::new(false);
