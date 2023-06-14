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

use crate::tests::test_helpers::sample_finalize_state;
use snarkvm::{
    console::{
        account::{Address, PrivateKey, ViewKey},
        network::{prelude::*, Testnet3},
        program::{Entry, Identifier, Literal, Plaintext, Value},
    },
    prelude::{Ledger, RecordsFilter, TestRng},
    synthesizer::{
        block::{Block, Transaction},
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
        synthesizer::{process::FinalizeGlobalState, store::helpers::memory::ConsensusMemory, Block},
    };

    use once_cell::sync::OnceCell;

    type CurrentNetwork = Testnet3;
    pub(crate) type CurrentLedger = Ledger<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;
    pub(crate) type CurrentConsensus = Consensus<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;

    pub(crate) fn sample_vm() -> VM<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
        VM::from(ConsensusStore::open(None).unwrap()).unwrap()
    }

    /// Samples a new finalize state.
    pub(crate) fn sample_finalize_state(block_height: u32) -> FinalizeGlobalState {
        FinalizeGlobalState::from(block_height, [0u8; 32])
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
                vm.genesis(&caller_private_key, rng).unwrap()
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
                vm.genesis(&private_key, rng).unwrap()
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
program test_program.aleo;

struct message:
    amount as u128;

record token:
    owner as address.private;
    amount as u64.private;

function compute:
    input r0 as message.private;
    input r1 as message.public;
    input r2 as message.private;
    input r3 as token.record;
    add r0.amount r1.amount into r4;
    cast r3.owner r3.amount into r5 as token.record;
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
                let microcredits = Identifier::from_str("microcredits").unwrap();
                let records = consensus
                    .ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| {
                        // TODO (raychu86): Find cleaner approach and check that the record is associated with the `credits.aleo` program
                        match record.data().get(&microcredits) {
                            Some(Entry::Private(Plaintext::Literal(Literal::U64(amount), _))) => !amount.is_zero(),
                            _ => false,
                        }
                    })
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);

                // Prepare the additional fee.
                let credits = records.values().next().unwrap().clone();
                let additional_fee = (credits, 6466000);

                // Deploy.
                let transaction =
                    consensus.ledger.vm().deploy(&caller_private_key, &program, additional_fee, None, rng).unwrap();
                // Verify.
                assert!(consensus.ledger.vm().verify_transaction(&transaction, None));
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
                let microcredits = Identifier::from_str("microcredits").unwrap();
                let records = consensus
                    .ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| {
                        // TODO (raychu86): Find cleaner approach and check that the record is associated with the `credits.aleo` program
                        match record.data().get(&microcredits) {
                            Some(Entry::Private(Plaintext::Literal(Literal::U64(amount), _))) => !amount.is_zero(),
                            _ => false,
                        }
                    })
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);
                // Select a record to spend.
                let record = records.values().next().unwrap().clone();

                // Prepare the fee.
                let fee = Some((record, 3000));

                // Retrieve the VM.
                let vm = consensus.ledger.vm();

                // Prepare the inputs.
                let inputs = [
                    Value::<CurrentNetwork>::from_str(&address.to_string()).unwrap(),
                    Value::<CurrentNetwork>::from_str("1u64").unwrap(),
                ]
                .into_iter();

                // Execute.
                let transaction =
                    vm.execute(&caller_private_key, ("credits.aleo", "mint"), inputs, fee, None, rng).unwrap();
                // Verify.
                assert!(vm.verify_transaction(&transaction, None));
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
    let genesis = vm.genesis(&private_key, rng).unwrap();

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

    // Compute a confirmed transactions to reuse later.
    let transactions = consensus.ledger.vm().speculate(sample_finalize_state(1), [transaction.clone()].iter()).unwrap();

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
    assert!(consensus.ledger.vm().finalize(sample_finalize_state(1), &transactions).is_err());
    // Ensure that the ledger deems the same transaction invalid.
    assert!(consensus.check_transaction_basic(&transaction, None).is_err());
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
    assert!(consensus.check_transaction_basic(&transaction, None).is_err());
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

    const NUM_GENESIS: usize = Block::<CurrentNetwork>::NUM_GENESIS_TRANSACTIONS;

    for height in 1..4 {
        println!("\nStarting on block {height}\n");

        // Fetch the unspent records.
        let microcredits = Identifier::from_str("microcredits").unwrap();
        let records: Vec<_> = consensus
            .ledger
            .find_records(&view_key, RecordsFilter::Unspent)
            .unwrap()
            .filter(|(_, record)| {
                // TODO (raychu86): Find cleaner approach and check that the record is associated with the `credits.aleo` program
                match record.data().get(&microcredits) {
                    Some(Entry::Private(Plaintext::Literal(Literal::U64(amount), _))) => !amount.is_zero(),
                    _ => false,
                }
            })
            .collect();
        assert_eq!(records.len(), NUM_GENESIS * (1 << (height - 1)));

        for (_, record) in records.iter() {
            // Prepare the inputs.
            let amount = match record.data().get(&Identifier::from_str("microcredits").unwrap()).unwrap() {
                Entry::Private(Plaintext::Literal(Literal::<CurrentNetwork>::U64(amount), _)) => amount,
                _ => unreachable!(),
            };
            let inputs = [Value::Record(record.clone()), Value::from_str(&format!("{}u64", **amount / 2)).unwrap()];
            // Create a new transaction.
            let transaction = consensus
                .ledger
                .vm()
                .execute(&private_key, ("credits.aleo", "split"), inputs.iter(), None, None, rng)
                .unwrap();
            // Add the transaction to the memory pool.
            consensus.add_unconfirmed_transaction(transaction).unwrap();
        }
        assert_eq!(consensus.memory_pool().num_unconfirmed_transactions(), NUM_GENESIS * (1 << (height - 1)));

        // Propose the next block.
        let next_block = consensus.propose_next_block(&private_key, rng).unwrap();

        // Ensure the block is a valid next block.
        consensus.check_next_block(&next_block).unwrap();
        // Construct a next block.
        consensus.advance_to_next_block(&next_block).unwrap();
        assert_eq!(consensus.ledger.latest_height(), height as u32);
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
        let prover_solution = consensus.coinbase_puzzle().prove(&epoch_challenge, address, rng.gen(), None).unwrap();

        // Check that the prover solution meets the proof target requirement.
        if prover_solution.to_target().unwrap() >= proof_target {
            assert!(consensus.add_unconfirmed_solution(&prover_solution).is_ok())
        } else {
            assert!(consensus.add_unconfirmed_solution(&prover_solution).is_err())
        }

        // Generate a prover solution with a minimum proof target.
        let prover_solution =
            consensus.coinbase_puzzle().prove(&epoch_challenge, address, rng.gen(), Some(proof_target));

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
        let prover_solution = match consensus.coinbase_puzzle().prove(
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
