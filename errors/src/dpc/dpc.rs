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

use crate::{
    algorithms::{
        CRHError,
        CommitmentError,
        EncodingError,
        EncryptionError,
        MerkleError,
        PRFError,
        SNARKError,
        SignatureError,
    },
    dpc::LedgerError,
    objects::AccountError,
    parameters::ParametersError,
};

#[derive(Debug, Error)]
pub enum DPCError {
    #[error("{}", _0)]
    AccountError(AccountError),

    #[error("{}", _0)]
    CommitmentError(CommitmentError),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    EncodingError(EncodingError),

    #[error("{}", _0)]
    EncryptionError(EncryptionError),

    #[error("{}", _0)]
    LedgerError(LedgerError),

    #[error("{}", _0)]
    MerkleError(MerkleError),

    #[error("{}", _0)]
    Message(String),

    #[error("missing inner snark proving parameters")]
    MissingInnerSnarkProvingParameters,

    #[error("missing outer snark proving parameters")]
    MissingOuterSnarkProvingParameters,

    #[error("{}", _0)]
    ParametersError(ParametersError),

    #[error("{}", _0)]
    PRFError(PRFError),

    #[error("{}", _0)]
    SignatureError(SignatureError),

    #[error("{}", _0)]
    SNARKError(SNARKError),
}

impl From<AccountError> for DPCError {
    fn from(error: AccountError) -> Self {
        DPCError::AccountError(error)
    }
}

impl From<CommitmentError> for DPCError {
    fn from(error: CommitmentError) -> Self {
        DPCError::CommitmentError(error)
    }
}

impl From<CRHError> for DPCError {
    fn from(error: CRHError) -> Self {
        DPCError::CRHError(error)
    }
}

impl From<EncodingError> for DPCError {
    fn from(error: EncodingError) -> Self {
        DPCError::EncodingError(error)
    }
}

impl From<EncryptionError> for DPCError {
    fn from(error: EncryptionError) -> Self {
        DPCError::EncryptionError(error)
    }
}

impl From<LedgerError> for DPCError {
    fn from(error: LedgerError) -> Self {
        DPCError::LedgerError(error)
    }
}

impl From<MerkleError> for DPCError {
    fn from(error: MerkleError) -> Self {
        DPCError::MerkleError(error)
    }
}

impl From<PRFError> for DPCError {
    fn from(error: PRFError) -> Self {
        DPCError::PRFError(error)
    }
}

impl From<SignatureError> for DPCError {
    fn from(error: SignatureError) -> Self {
        DPCError::SignatureError(error)
    }
}

impl From<ParametersError> for DPCError {
    fn from(error: ParametersError) -> Self {
        DPCError::ParametersError(error)
    }
}

impl From<SNARKError> for DPCError {
    fn from(error: SNARKError) -> Self {
        DPCError::SNARKError(error)
    }
}

impl From<std::io::Error> for DPCError {
    fn from(error: std::io::Error) -> Self {
        DPCError::Crate("std::io", format!("{:?}", error))
    }
}
