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

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

use anyhow::*;
use dyn_clone::DynClone;
use smallvec::SmallVec;
use snarkos_storage::Digest;
use snarkvm_algorithms::{
    merkle_tree::{MerklePath, MerkleTreeDigest},
    CommitmentScheme,
    MerkleParameters,
    SignatureScheme,
};
use snarkvm_dpc::{Block, LedgerScheme, TransactionScheme, testnet1::{Testnet1Components, Transaction}};

mod merkle;
pub use merkle::MerkleLedger;
mod indexed_merkle_tree;
pub use indexed_merkle_tree::IndexedMerkleTree;
use snarkvm_parameters::{LedgerMerkleTreeParameters, Parameter};
use snarkvm_utilities::{FromBytes, ToBytes};

pub trait Ledger: Send + Sync + DynClone {
    fn extend(
        &mut self,
        new_commitments: &[Digest],
        new_serial_numbers: &[Digest],
        new_memos: &[Digest],
    ) -> Result<Digest>;

    fn rollback(&mut self, commitments: &[Digest], serial_numbers: &[Digest], memos: &[Digest]) -> Result<()>;

    fn clear(&mut self);

    fn commitment_len(&self) -> usize;

    fn contains_commitment(&self, commitment: &Digest) -> bool;

    fn commitment_index(&self, commitment: &Digest) -> Option<usize>;

    fn contains_serial(&self, serial: &Digest) -> bool;

    fn contains_memo(&self, memo: &Digest) -> bool;

    fn validate_digest(&self, digest: &Digest) -> bool;

    fn digest(&self) -> Digest;

    fn generate_proof(&self, commitment: &Digest, index: usize) -> Result<Vec<(Digest, Digest)>>;

    /// checks if a ledgers state is consistent
    fn validate_ledger(&self) -> bool;
}

pub struct DynLedger(pub Box<dyn Ledger>);

impl Clone for DynLedger {
    fn clone(&self) -> Self {
        DynLedger(dyn_clone::clone_box(&*self.0))
    }
}

impl Deref for DynLedger {
    type Target = dyn Ledger;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for DynLedger {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub struct DeserializedLedger<'a, C: Testnet1Components> {
    parameters: Arc<C::MerkleParameters>,
    inner: &'a DynLedger,
    _p: PhantomData<C>,
}

impl DynLedger {
    pub fn deserialize<'a, C: Testnet1Components>(&'a self) -> DeserializedLedger<'a, C> {
        //todo: cache this
        let crh = <C::MerkleParameters as MerkleParameters>::H::from(
            FromBytes::read_le(&LedgerMerkleTreeParameters::load_bytes().expect("failed to load merkle parameters")[..])
                .expect("failed to read merkle parameters"),
        );
        let parameters = Arc::new(C::MerkleParameters::from(crh));

        DeserializedLedger {
            parameters,
            inner: self,
            _p: PhantomData,
        }
    }
}

impl<'a, C: Testnet1Components> DeserializedLedger<'a, C> {
    pub fn serialize(self) -> &'a DynLedger {
        self.inner
    }
}

impl<'a, C: Testnet1Components> LedgerScheme for DeserializedLedger<'a, C> {
    type Block = Block<Self::Transaction>;
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type MerkleParameters = C::MerkleParameters;
    type MerklePath = MerklePath<C::MerkleParameters>;
    type MerkleTreeDigest = MerkleTreeDigest<C::MerkleParameters>;
    type SerialNumber = <C::AccountSignature as SignatureScheme>::PublicKey;
    type Transaction = Transaction<C>;

    fn new(
        _path: Option<&Path>,
        _parameters: Arc<Self::MerkleParameters>,
        _genesis_block: Self::Block,
    ) -> Result<Self> {
        unimplemented!()
    }

    fn len(&self) -> usize {
        unimplemented!()
    }

    fn parameters(&self) -> &Arc<Self::MerkleParameters> {
        &self.parameters
    }

    fn digest(&self) -> Option<Self::MerkleTreeDigest> {
        <Self::MerkleTreeDigest as FromBytes>::read_le(&mut &self.inner.digest()[..]).ok()
    }

    fn validate_digest(&self, digest: &Self::MerkleTreeDigest) -> bool {
        let mut out = SmallVec::new();
        digest.write_le(&mut out).expect("failed to serialize commitment");
        let out = Digest(out);
        self.inner.validate_digest(&out)
    }

    fn contains_cm(&self, cm: &Self::Commitment) -> bool {
        let mut out = SmallVec::new();
        cm.write_le(&mut out).expect("failed to serialize commitment");
        let out = Digest(out);
        // match self.storage.contains_any_commitments_sync(&[out]) {
        //     Ok(value) => value,
        //     Err(e) => {
        //         //todo: should this be a panic since we really dont want the process to continue?
        //         error!("failed to check storage for commitment: {:?}", e);
        //         false
        //     },
        // }
        self.inner.contains_commitment(&out)
    }

    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool {
        let mut out = SmallVec::new();
        sn.write_le(&mut out).expect("failed to serialize serial");
        let out = Digest(out);
        // match self.storage.contains_any_serial_numbers_sync(&[out]) {
        //     Ok(value) => value,
        //     Err(e) => {
        //         error!("failed to check storage for serial: {:?}", e);
        //         false
        //     },
        // }
        self.inner.contains_serial(&out)
    }

    fn contains_memo(&self, memo: &<Self::Transaction as TransactionScheme>::Memorandum) -> bool {
        let mut out = SmallVec::new();
        memo.write_le(&mut out).expect("failed to serialize memo");
        let out = Digest(out);
        // match self.storage.contains_any_memos_sync(&[out]) {
        //     Ok(value) => value,
        //     Err(e) => {
        //         error!("failed to check storage for memo: {:?}", e);
        //         false
        //     },
        // }
        self.inner.contains_memo(&out)
    }

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<Self::MerklePath> {
        let mut out = SmallVec::new();
        cm.write_le(&mut out).expect("failed to serialize commitment");
        let out = Digest(out);
        // let cm_index = match self.storage.get_commitment_indexes_sync(&[out]) {
        //     Ok(indices) => indices.get(0).ok_or_else(|| anyhow!("missing commitment index"))?,
        //     Err(e) => {
        //         return Err(anyhow!("failed to check storage for commitment index: {:?}", e));
        //     },
        // };
        let cm_index = self
            .inner
            .commitment_index(&out)
            .ok_or_else(|| anyhow!("missing commitment index from ledger"))?;
        let path = self.inner.generate_proof(&out, cm_index)?;

        let mut out = Vec::with_capacity(path.len());
        for (left, right) in path {
            out.push((FromBytes::read_le(&left[..])?, FromBytes::read_le(&right[..])?));
        }

        Ok(MerklePath {
            parameters: self.parameters.clone(),
            path: out,
        })
    }

    fn verify_cm(
        _parameters: &Arc<Self::MerkleParameters>,
        digest: &Self::MerkleTreeDigest,
        cm: &Self::Commitment,
        witness: &Self::MerklePath,
    ) -> bool {
        witness.verify(digest, cm).unwrap()
    }
}
