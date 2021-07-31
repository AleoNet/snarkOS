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

use snarkvm::dpc::{testnet1::*, Account, AccountScheme, DPCScheme};

use rand::{CryptoRng, Rng};

pub fn setup_or_load_dpc<R: Rng + CryptoRng>(verify_only: bool, rng: &mut R) -> Testnet1DPC {
    match Testnet1DPC::load(verify_only) {
        Ok(dpc) => dpc,
        Err(err) => {
            println!("error - {}, re-running parameter Setup", err);
            Testnet1DPC::setup(rng).expect("DPC setup failed")
        }
    }
}

pub fn generate_test_accounts<R: Rng + CryptoRng>(rng: &mut R) -> [Account<Testnet1Parameters>; 3] {
    // TODO (howardwu): Remove DPCScheme<MerkleTreeLedger<S>> usage after decoupling ledger.
    let genesis_account = Account::<Testnet1Parameters>::new(rng).unwrap();
    let account_1 = Account::<Testnet1Parameters>::new(rng).unwrap();
    let account_2 = Account::<Testnet1Parameters>::new(rng).unwrap();

    [genesis_account, account_1, account_2]
}
