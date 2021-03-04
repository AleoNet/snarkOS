// Copyright (C) 2019-2021 Aleo Systems Inc.
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

#[derive(Debug, Error)]
pub enum DPCError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("Invalid number of inputs. (Current: {}, Max: {})", _0, _1)]
    InvalidNumberOfInputs(usize, usize),

    #[error("Invalid number of outputs. (Current: {}, Max: {})", _0, _1)]
    InvalidNumberOfOutputs(usize, usize),

    #[error("Missing transaction outputs)")]
    MissingOutputs,
}

impl From<anyhow::Error> for DPCError {
    fn from(error: anyhow::Error) -> Self {
        DPCError::Crate("anyhow", format!("{:?}", error))
    }
}

impl From<hex::FromHexError> for DPCError {
    fn from(error: hex::FromHexError) -> Self {
        DPCError::Crate("hex", format!("{:?}", error))
    }
}

impl From<snarkvm_objects::account::AccountError> for DPCError {
    fn from(error: snarkvm_objects::account::AccountError) -> Self {
        DPCError::Crate("snarkvm_objects::account", format!("{:?}", error))
    }
}

impl From<snarkvm_algorithms::errors::signature::SignatureError> for DPCError {
    fn from(error: snarkvm_algorithms::errors::signature::SignatureError) -> Self {
        DPCError::Crate("snarkvm_algorithms::errors::signature", format!("{:?}", error))
    }
}

impl From<snarkvm_algorithms::errors::crh::CRHError> for DPCError {
    fn from(error: snarkvm_algorithms::errors::crh::CRHError) -> Self {
        DPCError::Crate("snarkvm_algorithms::errors::crh", format!("{:?}", error))
    }
}

impl From<snarkvm_dpc::DPCError> for DPCError {
    fn from(error: snarkvm_dpc::DPCError) -> Self {
        DPCError::Crate("snarkvm_dpc", format!("{:?}", error))
    }
}

impl From<std::io::Error> for DPCError {
    fn from(error: std::io::Error) -> Self {
        DPCError::Crate("std::io", format!("{:?}", error))
    }
}
