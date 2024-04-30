// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::helpers::{Proposal, SignedProposals};

use snarkvm::{
    console::{account::Address, network::Network},
    prelude::{anyhow, bail, FromBytes, IoResult, Read, Result, ToBytes, Write},
};

use aleo_std::{aleo_ledger_dir, StorageMode};
use std::{fs, path::PathBuf};

// Returns the path where a proposal cache file may be stored.
pub fn proposal_cache_path(network: u16, dev: Option<u16>) -> PathBuf {
    // Obtain the path to the ledger.
    let mut path = aleo_ledger_dir(network, StorageMode::from(dev));
    // Go to the folder right above the ledger.
    path.pop();
    // Append the proposal store's file name.
    path.push(&format!(
        "current-proposal-cache-{network}{}",
        if let Some(id) = dev { format!("-{id}") } else { "".into() }
    ));

    path
}

/// A helper type for the cache of proposal and signed proposals.
pub struct ProposalCache<N: Network> {
    proposal: Option<Proposal<N>>,
    signed_proposals: SignedProposals<N>,
}

impl<N: Network> ProposalCache<N> {
    /// Initializes a new instance of the proposal cache.
    pub fn new(proposal: Option<Proposal<N>>, signed_proposals: SignedProposals<N>) -> Self {
        Self { proposal, signed_proposals }
    }

    /// Ensure that the proposal and every signed proposal is associated with the `expected_signer`.
    pub fn is_valid(&self, expected_signer: Address<N>) -> bool {
        self.proposal.as_ref().map(|proposal| proposal.batch_header().author() == expected_signer).unwrap_or(true)
            && self.signed_proposals.is_valid(expected_signer)
    }

    /// Returns `true` if a proposal cache exists for the given network and `dev`.
    pub fn exists(dev: Option<u16>) -> bool {
        proposal_cache_path(N::ID, dev).exists()
    }

    /// Load the proposal cache from the file system and ensure that the proposal cache is valid.
    pub fn load(expected_signer: Address<N>, dev: Option<u16>) -> Result<Self> {
        // Load the proposal cache from the file system.
        let proposal_path = proposal_cache_path(N::ID, dev);

        // Deserialize the proposal cache from the file system.
        let proposal_cache = match fs::read(&proposal_path) {
            Ok(bytes) => match Self::from_bytes_le(&bytes) {
                Ok(proposal_cache) => proposal_cache,
                Err(_) => bail!("Couldn't deserialize the proposal stored at {}", proposal_path.display()),
            },
            Err(_) => {
                bail!("Couldn't read the proposal stored at {}", proposal_path.display());
            }
        };

        // Ensure the proposal cache is valid.
        if !proposal_cache.is_valid(expected_signer) {
            bail!("The proposal cache is invalid for the given address {expected_signer}");
        }

        Ok(proposal_cache)
    }

    /// Store the proposal cache to the file system.
    pub fn store(&self, dev: Option<u16>) -> Result<()> {
        let path = proposal_cache_path(N::ID, dev);
        info!("Storing the proposal cache to {}...", path.display());

        // Serialize the proposal cache.
        let bytes = self.to_bytes_le()?;
        // Store the proposal cache to the file system.
        fs::write(&path, bytes)
            .map_err(|err| anyhow!("Couldn't write the proposal cache to {} - {err}", path.display()))?;

        Ok(())
    }

    /// Returns the proposal and signed proposals.
    pub fn into(self) -> (Option<Proposal<N>>, SignedProposals<N>) {
        (self.proposal, self.signed_proposals)
    }
}

impl<N: Network> ToBytes for ProposalCache<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Serialize the `proposal`.
        self.proposal.is_some().write_le(&mut writer)?;
        if let Some(proposal) = &self.proposal {
            proposal.write_le(&mut writer)?;
        }
        // Serialize the `signed_proposals`.
        self.signed_proposals.write_le(&mut writer)?;

        Ok(())
    }
}

impl<N: Network> FromBytes for ProposalCache<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Deserialize `proposal`.
        let has_proposal: bool = FromBytes::read_le(&mut reader)?;
        let proposal = match has_proposal {
            true => Some(Proposal::read_le(&mut reader)?),
            false => None,
        };
        // Deserialize `signed_proposals`.
        let signed_proposals = SignedProposals::read_le(&mut reader)?;

        Ok(Self::new(proposal, signed_proposals))
    }
}

impl<N: Network> Default for ProposalCache<N> {
    /// Initializes a new instance of the proposal cache.
    fn default() -> Self {
        Self::new(None, Default::default())
    }
}
