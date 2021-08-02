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

use crate::storage::*;
use snarkos_consensus::MerkleTreeLedger;
use snarkos_storage::LedgerStorage;
use snarkvm::{
    dpc::{testnet1::*, Account, AccountScheme, DPCScheme, NoopProgram},
    ledger::{Block, Storage},
    parameters::{testnet1::GenesisBlock, traits::genesis::Genesis},
    utilities::bytes::FromBytes,
};

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::{marker::PhantomData, sync::Arc};

pub static FIXTURE: Lazy<Fixture<LedgerStorage>> = Lazy::new(|| setup(false));
pub static FIXTURE_VK: Lazy<Fixture<LedgerStorage>> = Lazy::new(|| setup(true));

// helper for setting up e2e tests
pub struct Fixture<S: Storage> {
    pub dpc: Arc<Testnet1DPC>,
    pub test_accounts: [Account<Testnet1Parameters>; 3],
    pub genesis_block: Block<Testnet1Transaction>,
    pub program: NoopProgram<Testnet1Parameters>,
    pub rng: ChaChaRng,
    _storage: PhantomData<S>,
}

impl<S: Storage> Fixture<S> {
    pub fn ledger(&self) -> MerkleTreeLedger<S> {
        initialize_test_blockchain(self.genesis_block.clone())
    }
}

fn setup<S: Storage>(verify_only: bool) -> Fixture<S> {
    let mut rng = ChaChaRng::seed_from_u64(1231275789u64);

    // Setup or load DPC.
    let dpc = match Testnet1DPC::load(verify_only) {
        Ok(dpc) => dpc,
        Err(err) => {
            println!("error - {}, re-running parameter Setup", err);
            Testnet1DPC::setup(&mut rng).expect("DPC setup failed")
        }
    };
    let program = dpc.noop_program.clone();

    // Generate addresses.
    let account_0 = Account::<Testnet1Parameters>::new(&mut rng).unwrap();
    let account_1 = Account::<Testnet1Parameters>::new(&mut rng).unwrap();
    let account_2 = Account::<Testnet1Parameters>::new(&mut rng).unwrap();
    let test_accounts = [account_0, account_1, account_2];

    Fixture {
        dpc: Arc::new(dpc),
        test_accounts,
        genesis_block: FromBytes::read_le(GenesisBlock::load_bytes().as_slice()).unwrap(),
        program,
        rng,
        _storage: PhantomData,
    }
}
