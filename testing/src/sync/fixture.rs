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

use crate::{
    dpc::{generate_test_accounts, setup_or_load_parameters},
    sync::genesis,
};
use snarkos_consensus::{DynLedger, MerkleLedger};
use snarkos_storage::{key_value::KeyValueStore, DynStorage, MemDb};
use snarkvm_algorithms::{MerkleParameters, CRH};
use snarkvm_dpc::{
    testnet1::{instantiated::*, NoopProgram, Testnet1Components},
    Account,
    Block,
};
use snarkvm_parameters::{LedgerMerkleTreeParameters, Parameter};
use snarkvm_utilities::bytes::FromBytes;

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::sync::Arc;

pub static FIXTURE: Lazy<Fixture> = Lazy::new(|| setup(false));
pub static FIXTURE_VK: Lazy<Fixture> = Lazy::new(|| setup(true));

// helper for setting up e2e tests
pub struct Fixture {
    pub dpc: Arc<Testnet1DPC>,
    pub test_accounts: [Account<Components>; 3],
    pub ledger_parameters: Arc<CommitmentMerkleParameters>,
    pub genesis_block: Block<Testnet1Transaction>,
    pub program: NoopProgram<Components>,
    pub rng: ChaChaRng,
}

impl Fixture {
    pub fn storage(&self) -> DynStorage {
        Arc::new(KeyValueStore::new(MemDb::new()))
    }

    pub fn ledger(&self) -> DynLedger {
        let ledger_parameters = {
            type Parameters = <Components as Testnet1Components>::MerkleParameters;
            let parameters: <<Parameters as MerkleParameters>::H as CRH>::Parameters =
                FromBytes::read_le(&LedgerMerkleTreeParameters::load_bytes().unwrap()[..]).unwrap();
            let crh = <Parameters as MerkleParameters>::H::from(parameters);
            Arc::new(Parameters::from(crh))
        };

        DynLedger(Box::new(
            MerkleLedger::new(ledger_parameters, &[], &[], &[], &[]).unwrap(),
        ))
    }
}

fn setup(verify_only: bool) -> Fixture {
    let mut rng = ChaChaRng::seed_from_u64(1231275789u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, dpc) = setup_or_load_parameters::<_>(verify_only, &mut rng);

    // Generate addresses
    let test_accounts = generate_test_accounts::<_>(&dpc, &mut rng);

    let genesis_block: Block<Testnet1Transaction> = genesis();

    let program = dpc.noop_program.clone();

    let dpc = Arc::new(dpc);

    Fixture {
        dpc,
        test_accounts,
        ledger_parameters,
        genesis_block,
        program,
        rng,
    }
}
