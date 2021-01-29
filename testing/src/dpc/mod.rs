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

use snarkos_consensus::MerkleTreeLedger;
use snarkvm_dpc::base_dpc::{instantiated::*, parameters::PublicParameters};
use snarkvm_models::{
    algorithms::{MerkleParameters, CRH},
    dpc::DPCScheme,
    objects::AccountScheme,
    parameters::Parameters,
};
use snarkvm_objects::Account;
use snarkvm_parameters::LedgerMerkleTreeParameters;
use snarkvm_utilities::bytes::FromBytes;

use rand::Rng;

pub fn setup_or_load_parameters<R: Rng>(
    verify_only: bool,
    rng: &mut R,
) -> (
    CommitmentMerkleParameters,
    <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
) {
    // TODO (howardwu): Resolve this inconsistency on import structure with a new model once MerkleParameters are refactored.
    let crh_parameters =
        <MerkleTreeCRH as CRH>::Parameters::read(&LedgerMerkleTreeParameters::load_bytes().unwrap()[..])
            .expect("read bytes as hash for MerkleParameters in ledger");
    let merkle_tree_hash_parameters = <CommitmentMerkleParameters as MerkleParameters>::H::from(crh_parameters);
    let ledger_merkle_tree_parameters = From::from(merkle_tree_hash_parameters);

    let parameters = match <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters::load(verify_only) {
        Ok(parameters) => parameters,
        Err(err) => {
            println!("error - {}, re-running parameter Setup", err);
            <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::setup(&ledger_merkle_tree_parameters, rng)
                .expect("DPC setup failed")
        }
    };

    (ledger_merkle_tree_parameters, parameters)
}

pub fn load_verifying_parameters() -> PublicParameters<Components> {
    PublicParameters::<Components>::load_vk_direct().unwrap()
}

pub fn generate_test_accounts<R: Rng>(
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    rng: &mut R,
) -> [Account<Components>; 3] {
    let signature_parameters = &parameters.system_parameters.account_signature;
    let commitment_parameters = &parameters.system_parameters.account_commitment;
    let encryption_parameters = &parameters.system_parameters.account_encryption;

    let genesis_account =
        Account::new(signature_parameters, commitment_parameters, encryption_parameters, rng).unwrap();
    let account_1 = Account::new(signature_parameters, commitment_parameters, encryption_parameters, rng).unwrap();
    let account_2 = Account::new(signature_parameters, commitment_parameters, encryption_parameters, rng).unwrap();

    [genesis_account, account_1, account_2]
}
