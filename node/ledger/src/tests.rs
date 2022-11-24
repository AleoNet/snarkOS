// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{tests::test_helpers::CurrentLedger, Ledger};
use snarkvm::{
    console::network::{prelude::*, Testnet3},
    prelude::TestRng,
    synthesizer::{block::Block, store::ConsensusStore, vm::VM, ConsensusMemory},
};

type CurrentNetwork = Testnet3;

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use snarkvm::{
        console::{account::PrivateKey, network::Testnet3},
        prelude::TestRng,
        synthesizer::ConsensusMemory,
    };

    use once_cell::sync::OnceCell;

    type CurrentNetwork = Testnet3;
    pub(crate) type CurrentLedger = Ledger<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;

    pub(crate) fn sample_genesis_private_key(rng: &mut TestRng) -> PrivateKey<CurrentNetwork> {
        static INSTANCE: OnceCell<PrivateKey<CurrentNetwork>> = OnceCell::new();
        *INSTANCE.get_or_init(|| {
            // Initialize a new caller.
            PrivateKey::<CurrentNetwork>::new(rng).unwrap()
        })
    }
}

fn sample_genesis_block() -> Block<CurrentNetwork> {
    Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap()
}

#[test]
fn test_load() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    // Initialize the store.
    let store = ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap();
    // Create a genesis block.
    let genesis = Block::genesis(&VM::from(store).unwrap(), &private_key, rng).unwrap();

    // Initialize the ledger with the genesis block.
    let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
    assert_eq!(ledger.latest_hash(), genesis.hash());
    assert_eq!(ledger.latest_height(), genesis.height());
    assert_eq!(ledger.latest_round(), genesis.round());
    assert_eq!(ledger.latest_block(), genesis);
}

#[test]
fn test_load_unchecked() {
    // Load the genesis block.
    let genesis = sample_genesis_block();

    // Initialize the ledger without checks.
    let ledger = CurrentLedger::load_unchecked(genesis.clone(), None).unwrap();
    assert_eq!(ledger.latest_hash(), genesis.hash());
    assert_eq!(ledger.latest_height(), genesis.height());
    assert_eq!(ledger.latest_round(), genesis.round());
    assert_eq!(ledger.latest_block(), genesis);

    // Initialize the ledger with the genesis block.
    let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
    assert_eq!(ledger.latest_hash(), genesis.hash());
    assert_eq!(ledger.latest_height(), genesis.height());
    assert_eq!(ledger.latest_round(), genesis.round());
    assert_eq!(ledger.latest_block(), genesis);
}

#[test]
fn test_state_path() {
    // Load the genesis block.
    let genesis = sample_genesis_block();
    // Initialize the ledger with the genesis block.
    let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
    // Retrieve the genesis block.
    let block = ledger.get_block(0).unwrap();
    assert_eq!(genesis, block);

    // Construct the state path.
    let commitments = block.transactions().commitments().collect::<Vec<_>>();
    let commitment = commitments[0];

    let _state_path = ledger.get_state_path_for_commitment(commitment).unwrap();
}
