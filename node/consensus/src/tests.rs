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

use snarkos_node_ledger::{Ledger, RecordsFilter};
use snarkvm::{
    console::{
        account::{Address, PrivateKey, ViewKey},
        network::{prelude::*, Testnet3},
        program::{Identifier, ProgramID, Value},
    },
    prelude::TestRng,
    synthesizer::{
        block::{Block, Transaction, Transactions},
        program::Program,
        store::ConsensusStore,
        vm::VM,
    },
};

use tracing_test::traced_test;

use indexmap::IndexMap;

type CurrentNetwork = Testnet3;

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::Consensus;
    use snarkvm::{
        console::{account::PrivateKey, network::Testnet3, program::Value},
        prelude::TestRng,
        synthesizer::{Block, ConsensusMemory},
    };

    use once_cell::sync::OnceCell;

    type CurrentNetwork = Testnet3;
    pub(crate) type CurrentLedger = Ledger<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;
    pub(crate) type CurrentConsensus = Consensus<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;

    pub(crate) fn sample_vm() -> VM<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
        VM::from(ConsensusStore::open(None).unwrap()).unwrap()
    }

    pub(crate) fn sample_genesis_private_key(rng: &mut TestRng) -> PrivateKey<CurrentNetwork> {
        static INSTANCE: OnceCell<PrivateKey<CurrentNetwork>> = OnceCell::new();
        *INSTANCE.get_or_init(|| {
            // Initialize a new caller.
            PrivateKey::<CurrentNetwork>::new(rng).unwrap()
        })
    }

    #[allow(dead_code)]
    pub(crate) fn sample_genesis_block(rng: &mut TestRng) -> Block<CurrentNetwork> {
        static INSTANCE: OnceCell<Block<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the VM.
                let vm = crate::tests::test_helpers::sample_vm();
                // Initialize a new caller.
                let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
                // Return the block.
                Block::genesis(&vm, &caller_private_key, rng).unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_genesis_block_with_private_key(
        rng: &mut TestRng,
        private_key: PrivateKey<CurrentNetwork>,
    ) -> Block<CurrentNetwork> {
        static INSTANCE: OnceCell<Block<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the VM.
                let vm = crate::tests::test_helpers::sample_vm();
                // Return the block.
                Block::genesis(&vm, &private_key, rng).unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_genesis_consensus(rng: &mut TestRng) -> CurrentConsensus {
        // Sample the genesis private key.
        let private_key = sample_genesis_private_key(rng);
        // Sample the genesis block.
        let genesis = sample_genesis_block_with_private_key(rng, private_key);

        // Initialize the ledger with the genesis block and the associated private key.
        let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
        assert_eq!(0, ledger.latest_height());
        assert_eq!(genesis.hash(), ledger.latest_hash());
        assert_eq!(genesis.round(), ledger.latest_round());
        assert_eq!(genesis, ledger.get_block(0).unwrap());

        CurrentConsensus::new(ledger, true).unwrap()
    }

    pub(crate) fn sample_program() -> Program<CurrentNetwork> {
        static INSTANCE: OnceCell<Program<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize a new program.
                Program::<CurrentNetwork>::from_str(
                    r"
program testing.aleo;

struct message:
    amount as u128;

record token:
    owner as address.private;
    gates as u64.private;
    amount as u64.private;

function compute:
    input r0 as message.private;
    input r1 as message.public;
    input r2 as message.private;
    input r3 as token.record;
    add r0.amount r1.amount into r4;
    cast r3.owner r3.gates r3.amount into r5 as token.record;
    output r4 as u128.public;
    output r5 as token.record;",
                )
                .unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_deployment_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
        static INSTANCE: OnceCell<Transaction<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the program.
                let program = sample_program();

                // Initialize a new caller.
                let caller_private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
                let caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();

                // Initialize the consensus.
                let consensus = crate::tests::test_helpers::sample_genesis_consensus(rng);

                // Fetch the unspent records.
                let records = consensus
                    .ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| !record.gates().is_zero())
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);

                // Prepare the additional fee.
                let credits = records.values().next().unwrap().clone();
                let additional_fee = (credits, 10);

                // Deploy.
                let transaction = Transaction::deploy(
                    consensus.ledger.vm(),
                    &caller_private_key,
                    &program,
                    additional_fee,
                    None,
                    rng,
                )
                .unwrap();
                // Verify.
                assert!(consensus.ledger.vm().verify(&transaction));
                // Return the transaction.
                transaction
            })
            .clone()
    }

    pub(crate) fn sample_execution_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
        static INSTANCE: OnceCell<Transaction<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize a new caller.
                let caller_private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
                let caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
                let address = Address::try_from(&caller_private_key).unwrap();

                // Initialize the consensus.
                let consensus = crate::tests::test_helpers::sample_genesis_consensus(rng);

                // Fetch the unspent records.
                let records = consensus
                    .ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| !record.gates().is_zero())
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);
                // Select a record to spend.
                let record = records.values().next().unwrap().clone();

                // Retrieve the VM.
                let vm = consensus.ledger.vm();

                // Authorize.
                let authorization = vm
                    .authorize(
                        &caller_private_key,
                        ProgramID::from_str("credits.aleo").unwrap(),
                        Identifier::from_str("transfer").unwrap(),
                        &[
                            Value::<CurrentNetwork>::Record(record),
                            Value::<CurrentNetwork>::from_str(&address.to_string()).unwrap(),
                            Value::<CurrentNetwork>::from_str("1u64").unwrap(),
                        ],
                        rng,
                    )
                    .unwrap();
                assert_eq!(authorization.len(), 1);

                // Execute.
                let transaction = Transaction::execute_authorization(vm, authorization, None, rng).unwrap();
                // Verify.
                assert!(vm.verify(&transaction));
                // Return the transaction.
                transaction
            })
            .clone()
    }
}

#[test]
fn test_validators() {
    // Initialize an RNG.
    let rng = &mut TestRng::default();

    // Sample the private key, view key, and address.
    let private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let view_key = ViewKey::try_from(private_key).unwrap();
    let address = Address::try_from(&view_key).unwrap();

    // Initialize the VM.
    let vm = crate::tests::test_helpers::sample_vm();

    // Create a genesis block.
    let genesis = Block::genesis(&vm, &private_key, rng).unwrap();

    // Initialize the validators.
    let validators: IndexMap<Address<_>, ()> = [(address, ())].into_iter().collect();

    // Ensure the block is signed by an authorized validator.
    let signer = genesis.signature().to_address();
    if !validators.contains_key(&signer) {
        let validator = validators.iter().next().unwrap().0;
        eprintln!("{} {} {} {}", *validator, signer, *validator == signer, validators.contains_key(&signer));
        eprintln!(
            "Block {} ({}) is signed by an unauthorized validator ({})",
            genesis.height(),
            genesis.hash(),
            signer
        );
    }
    assert!(validators.contains_key(&signer));
}

#[test]
#[traced_test]
fn test_ledger_deploy() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    // Sample the genesis consensus.
    let consensus = test_helpers::sample_genesis_consensus(rng);

    // Add a transaction to the memory pool.
    let transaction = crate::tests::test_helpers::sample_deployment_transaction(rng);
    consensus.add_unconfirmed_transaction(transaction.clone()).unwrap();

    // Propose the next block.
    let next_block = consensus.propose_next_block(&private_key, rng).unwrap();

    // Ensure the block is a valid next block.
    consensus.check_next_block(&next_block).unwrap();

    // Construct a next block.
    consensus.advance_to_next_block(&next_block).unwrap();
    assert_eq!(consensus.ledger.latest_height(), 1);
    assert_eq!(consensus.ledger.latest_hash(), next_block.hash());
    assert!(consensus.ledger.contains_transaction_id(&transaction.id()).unwrap());
    assert!(transaction.input_ids().count() > 0);
    assert!(consensus.ledger.contains_input_id(transaction.input_ids().next().unwrap()).unwrap());

    // Ensure that the VM can't re-deploy the same program.
    assert!(consensus.ledger.vm().finalize(&Transactions::from(&[transaction.clone()])).is_err());
    // Ensure that the ledger deems the same transaction invalid.
    assert!(consensus.check_transaction_basic(&transaction).is_err());
    // Ensure that the ledger cannot add the same transaction.
    assert!(consensus.add_unconfirmed_transaction(transaction).is_err());
}

#[test]
#[traced_test]
fn test_ledger_execute() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    // Sample the genesis consensus.
    let consensus = test_helpers::sample_genesis_consensus(rng);

    // Add a transaction to the memory pool.
    let transaction = crate::tests::test_helpers::sample_execution_transaction(rng);
    consensus.add_unconfirmed_transaction(transaction.clone()).unwrap();

    // Propose the next block.
    let next_block = consensus.propose_next_block(&private_key, rng).unwrap();

    // Ensure the block is a valid next block.
    consensus.check_next_block(&next_block).unwrap();

    // Construct a next block.
    consensus.advance_to_next_block(&next_block).unwrap();
    assert_eq!(consensus.ledger.latest_height(), 1);
    assert_eq!(consensus.ledger.latest_hash(), next_block.hash());

    // Ensure that the ledger deems the same transaction invalid.
    assert!(consensus.check_transaction_basic(&transaction).is_err());
    // Ensure that the ledger cannot add the same transaction.
    assert!(consensus.add_unconfirmed_transaction(transaction).is_err());
}

#[test]
#[traced_test]
fn test_ledger_execute_many() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key, view key, and address.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    let view_key = ViewKey::try_from(private_key).unwrap();

    // Sample the genesis consensus.
    let consensus = crate::tests::test_helpers::sample_genesis_consensus(rng);

    for height in 1..6 {
        // Fetch the unspent records.
        let records: Vec<_> = consensus
            .ledger
            .find_records(&view_key, RecordsFilter::Unspent)
            .unwrap()
            .filter(|(_, record)| !record.gates().is_zero())
            .collect();
        assert_eq!(records.len(), 1 << (height - 1));

        for (_, record) in records {
            // Prepare the inputs.
            let inputs =
                [Value::Record(record.clone()), Value::from_str(&format!("{}u64", ***record.gates() / 2)).unwrap()];
            // Create a new transaction.
            let transaction = Transaction::execute(
                consensus.ledger.vm(),
                &private_key,
                ProgramID::from_str("credits.aleo").unwrap(),
                Identifier::from_str("split").unwrap(),
                inputs.iter(),
                None,
                None,
                rng,
            )
            .unwrap();
            // Add the transaction to the memory pool.
            consensus.add_unconfirmed_transaction(transaction).unwrap();
        }
        assert_eq!(consensus.memory_pool().num_unconfirmed_transactions(), 1 << (height - 1));

        // Propose the next block.
        let next_block = consensus.propose_next_block(&private_key, rng).unwrap();

        // Ensure the block is a valid next block.
        consensus.check_next_block(&next_block).unwrap();
        // Construct a next block.
        consensus.advance_to_next_block(&next_block).unwrap();
        assert_eq!(consensus.ledger.latest_height(), height);
        assert_eq!(consensus.ledger.latest_hash(), next_block.hash());
    }
}

#[test]
#[traced_test]
fn test_proof_target() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key and address.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    let address = Address::try_from(&private_key).unwrap();

    // Sample the genesis consensus.
    let consensus = crate::tests::test_helpers::sample_genesis_consensus(rng);

    // Fetch the proof target and epoch challenge for the block.
    let proof_target = consensus.ledger.latest_proof_target();
    let epoch_challenge = consensus.ledger.latest_epoch_challenge().unwrap();

    for _ in 0..100 {
        // Generate a prover solution.
        let prover_solution = consensus.coinbase_puzzle.prove(&epoch_challenge, address, rng.gen(), None).unwrap();

        // Check that the prover solution meets the proof target requirement.
        if prover_solution.to_target().unwrap() >= proof_target {
            assert!(consensus.add_unconfirmed_solution(&prover_solution).is_ok())
        } else {
            assert!(consensus.add_unconfirmed_solution(&prover_solution).is_err())
        }

        // Generate a prover solution with a minimum proof target.
        let prover_solution = consensus.coinbase_puzzle.prove(&epoch_challenge, address, rng.gen(), Some(proof_target));

        // Check that the prover solution meets the proof target requirement.
        if let Ok(prover_solution) = prover_solution {
            assert!(prover_solution.to_target().unwrap() >= proof_target);
            assert!(consensus.add_unconfirmed_solution(&prover_solution).is_ok())
        }
    }
}

#[test]
#[traced_test]
fn test_coinbase_target() {
    let rng = &mut TestRng::default();

    // Sample the genesis private key and address.
    let private_key = crate::tests::test_helpers::sample_genesis_private_key(rng);
    let address = Address::try_from(&private_key).unwrap();

    // Sample the genesis consensus.
    let consensus = test_helpers::sample_genesis_consensus(rng);

    // Add a transaction to the memory pool.
    let transaction = crate::tests::test_helpers::sample_execution_transaction(rng);
    consensus.add_unconfirmed_transaction(transaction).unwrap();

    // Ensure that the ledger can't create a block that satisfies the coinbase target.
    let proposed_block = consensus.propose_next_block(&private_key, rng).unwrap();
    // Ensure the block does not contain a coinbase solution.
    assert!(proposed_block.coinbase().is_none());

    // Check that the ledger won't generate a block for a cumulative target that does not meet the requirements.
    let mut cumulative_target = 0u128;
    let epoch_challenge = consensus.ledger.latest_epoch_challenge().unwrap();

    while cumulative_target < consensus.ledger.latest_coinbase_target() as u128 {
        // Generate a prover solution.
        let prover_solution = match consensus.coinbase_puzzle.prove(
            &epoch_challenge,
            address,
            rng.gen(),
            Some(consensus.ledger.latest_proof_target()),
        ) {
            Ok(prover_solution) => prover_solution,
            Err(_) => continue,
        };

        // Try to add the prover solution to the memory pool.
        if consensus.add_unconfirmed_solution(&prover_solution).is_ok() {
            // Add to the cumulative target if the prover solution is valid.
            cumulative_target += prover_solution.to_target().unwrap() as u128;
        }
    }

    // Ensure that the ledger can create a block that satisfies the coinbase target.
    let proposed_block = consensus.propose_next_block(&private_key, rng).unwrap();
    // Ensure the block contains a coinbase solution.
    assert!(proposed_block.coinbase().is_some());
}
