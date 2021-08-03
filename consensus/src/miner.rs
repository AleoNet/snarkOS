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

use crate::{error::ConsensusError, Consensus};
use snarkos_storage::{SerialBlock, SerialBlockHeader, SerialRecord, SerialTransaction};
use snarkvm_dpc::{testnet1::instantiated::*, Address, BlockHeader, BlockHeaderHash, DPCComponents, ProgramScheme};
use snarkvm_posw::{txids_to_roots, PoswMarlin};
use snarkvm_utilities::{to_bytes_le, ToBytes};

use chrono::Utc;
use rand::thread_rng;
use std::sync::Arc;

lazy_static::lazy_static! {
    /// The mining instance that is initialized with a proving key.
    static ref POSW: PoswMarlin = {
        PoswMarlin::load().expect("could not instantiate the miner")
    };
}

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
pub struct MineContext {
    /// The coinbase address that mining rewards are assigned to.
    address: Address<Components>,
    /// The sync parameters for the network of this miner.
    pub consensus: Arc<Consensus>,
    block_height: usize,
    canon_header: SerialBlockHeader,
}

impl MineContext {
    pub async fn prepare(address: Address<Components>, consensus: Arc<Consensus>) -> Result<Self, ConsensusError> {
        let canon = consensus.storage.canon().await?;

        let canon_header = consensus.storage.get_block_header(&canon.hash).await?;

        Ok(Self {
            address,
            consensus,
            block_height: canon.block_height,
            canon_header,
        })
    }

    /// Add a coinbase transaction to a list of candidate block transactions
    pub async fn add_coinbase_transaction(
        &self,
        transactions: &mut Vec<SerialTransaction>,
    ) -> Result<Vec<SerialRecord>, ConsensusError> {
        let program_vk_hash = self.consensus.dpc.noop_program.id();

        let new_birth_programs = vec![program_vk_hash.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_death_programs = vec![program_vk_hash.clone(); Components::NUM_OUTPUT_RECORDS];

        for transaction in transactions.iter() {
            if self.consensus.parameters.network_id != transaction.network {
                return Err(ConsensusError::ConflictingNetworkId(
                    self.consensus.parameters.network_id.id(),
                    transaction.network.id(),
                ));
            }
        }

        //(records, tx)
        let response = self
            .consensus
            .create_coinbase_transaction(
                self.block_height as u32 + 1,
                transactions,
                program_vk_hash,
                new_birth_programs,
                new_death_programs,
                vec![self.address.clone(); Components::NUM_OUTPUT_RECORDS]
                    .into_iter()
                    .map(|x| x.into())
                    .collect(),
            )
            .await?;

        transactions.push(response.transaction);
        Ok(response.records)
    }

    /// Acquires the storage lock and returns the previous block header and verified transactions.
    #[allow(clippy::type_complexity)]
    pub async fn establish_block(
        &self,
        mut transactions: Vec<SerialTransaction>,
    ) -> Result<(Vec<SerialTransaction>, Vec<SerialRecord>), ConsensusError> {
        let coinbase_records = self.add_coinbase_transaction(&mut transactions).await?;

        // Verify transactions
        // assert!(Testnet1DPC::verify_transactions(
        //     &self.consensus.public_parameters,
        //     &transactions,

        // )?);

        Ok((transactions, coinbase_records))
    }

    /// Run proof of work to find block.
    /// Returns BlockHeader with nonce solution.
    pub fn find_block(
        &self,
        transactions: &[SerialTransaction],
        parent_header: &SerialBlockHeader,
    ) -> Result<SerialBlockHeader, ConsensusError> {
        let txids = transactions.iter().map(|x| x.id).collect::<Vec<_>>();
        let (merkle_root_hash, pedersen_merkle_root_hash, subroots) = txids_to_roots(&txids);

        let time = Utc::now().timestamp();
        let difficulty_target = self.consensus.parameters.get_block_difficulty(parent_header, time);

        // TODO: Switch this to use a user-provided RNG
        let (nonce, proof) = POSW.mine(
            &subroots,
            difficulty_target,
            &mut thread_rng(),
            self.consensus.parameters.max_nonce,
        )?;

        Ok(BlockHeader {
            previous_block_hash: BlockHeaderHash(parent_header.hash().bytes().unwrap()),
            merkle_root_hash,
            pedersen_merkle_root_hash,
            time,
            difficulty_target,
            nonce,
            proof: proof.into(),
        }
        .into())
    }

    /// Returns a mined block.
    /// Calls methods to fetch transactions, run proof of work, and add the block into the chain for storage.
    pub async fn mine_block(&self) -> Result<(SerialBlock, Vec<SerialRecord>), ConsensusError> {
        let candidate_transactions = self.consensus.fetch_memory_pool().await;
        debug!("Miner@{}: creating a block", self.block_height);

        let (transactions, coinbase_records) = self.establish_block(candidate_transactions).await?;

        debug!("Miner@{}: generated a coinbase transaction", self.block_height);

        for (index, record) in coinbase_records.iter().enumerate() {
            let record_commitment = hex::encode(&to_bytes_le![record.commitment]?);
            debug!(
                "Miner@{}: Coinbase record {:?} commitment: {:?}",
                self.block_height, index, record_commitment
            );
        }

        let header = self.find_block(&transactions, &self.canon_header)?;

        debug!("Miner@{}: found a block", self.block_height);

        let block = SerialBlock { header, transactions };

        //todo: remove this
        self.consensus.receive_block(block.clone()).await;

        // Store the non-dummy coinbase records.
        let mut records_to_store = vec![];
        for record in &coinbase_records {
            if !record.is_dummy {
                records_to_store.push(record.clone());
            }
        }
        self.consensus.storage.store_records(&records_to_store).await?;

        Ok((block, coinbase_records))
    }
}
