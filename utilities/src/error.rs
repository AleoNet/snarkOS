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
