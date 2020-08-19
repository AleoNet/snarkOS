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

use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum ParametersError {
    #[error("expected checksum of {}, found checksum of {}", _0, _1)]
    ChecksumMismatch(String, String),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Remote fetch is disabled, enable compiler flag for feature")]
    RemoteFetchDisabled,
}

#[cfg(any(test, feature = "remote"))]
impl From<curl::Error> for ParametersError {
    fn from(error: curl::Error) -> Self {
        ParametersError::Crate("curl::error", format!("{:?}", error))
    }
}

impl From<std::io::Error> for ParametersError {
    fn from(error: std::io::Error) -> Self {
        ParametersError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::path::StripPrefixError> for ParametersError {
    fn from(error: std::path::StripPrefixError) -> Self {
        ParametersError::Crate("std::path", format!("{:?}", error))
    }
}

impl From<ParametersError> for std::io::Error {
    fn from(error: ParametersError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
    }
}
