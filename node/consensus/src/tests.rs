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
use indexmap::IndexMap;
use narwhal_types::TransactionProto;
use rand::seq::IteratorRandom;
use snarkos_account::Account;
use snarkos_node::Validator;
use snarkos_node_messages::{Data, Message, UnconfirmedTransaction};
use snarkvm::{
    console::{
        account::{Address, PrivateKey, ViewKey},
        network::{prelude::*, Testnet3},
        program::{Entry, Identifier, Literal, Plaintext, Value},
    },
    prelude::{Ledger, RecordsFilter, TestRng},
    synthesizer::{
        block::{Block, Transaction},
        store::{helpers::memory::ConsensusMemory, ConsensusStore},
        vm::VM,
    },
};
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_test::traced_test;

type CurrentNetwork = Testnet3;
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::Consensus;
    use snarkvm::{
        console::{account::PrivateKey, network::Testnet3, program::Value},
        prelude::TestRng,
        synthesizer::{process::FinalizeGlobalState, store::helpers::memory::ConsensusMemory, Block, Program},
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

    pub(crate) fn start_logger(default_level: LevelFilter) {
        let filter = match EnvFilter::try_from_default_env() {
            Ok(filter) => filter
                .add_directive("anemo=off".parse().unwrap())
                .add_directive("tokio_util=off".parse().unwrap())
                .add_directive("narwhal_config=off".parse().unwrap())
                .add_directive("narwhal_consensus=off".parse().unwrap())
                .add_directive("narwhal_executor=off".parse().unwrap())
                .add_directive("narwhal_network=off".parse().unwrap())
                .add_directive("narwhal_primary=off".parse().unwrap())
                .add_directive("narwhal_worker=off".parse().unwrap()),
            _ => EnvFilter::default()
                .add_directive(default_level.into())
                .add_directive("anemo=off".parse().unwrap())
                .add_directive("tokio_util=off".parse().unwrap())
                .add_directive("narwhal_config=off".parse().unwrap())
                .add_directive("narwhal_consensus=off".parse().unwrap())
                .add_directive("narwhal_executor=off".parse().unwrap())
                .add_directive("narwhal_network=off".parse().unwrap())
                .add_directive("narwhal_primary=off".parse().unwrap())
                .add_directive("narwhal_worker=off".parse().unwrap()),
        };

        tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "This test is intended to be run on-demand and in isolation."]
async fn test_bullshark_full() {
    // Start the logger.
    test_helpers::start_logger(LevelFilter::INFO);

    // TODO: introduce a Ctrl-C signal handler that will delete the temporary databases.

    // The number of validators to run.
    // TODO: support a different number than 4.
    const N_VALIDATORS: u16 = 4;

    // The randomly-seeded source of deterministic randomness.
    let mut rng = TestRng::default();

    // Sample the genesis private key.
    let genesis_private_key = test_helpers::sample_genesis_private_key(&mut rng);
    let genesis_address = Address::try_from(&genesis_private_key).unwrap();

    // Sample the genesis block.
    let genesis = test_helpers::sample_genesis_block_with_private_key(&mut rng, genesis_private_key);

    // Collect the validator addresses.
    let mut validator_addrs = vec![];
    for i in 0..N_VALIDATORS {
        let addr: SocketAddr = format!("127.0.0.1:{}", 4130 + i).parse().unwrap();
        validator_addrs.push(addr);
    }

    // Start and collect the validator nodes.
    let mut validators = vec![];
    for (i, addr) in validator_addrs.iter().copied().enumerate() {
        info!("Staring validator {i} at {addr}.");

        let account = Account::<CurrentNetwork>::new(&mut rng).unwrap();
        let other_addrs = validator_addrs.iter().copied().filter(|&a| a != addr).collect::<Vec<_>>();
        let validator = Validator::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::new(
            addr,
            None,
            account,
            &other_addrs,    // the other validators are trusted peers
            genesis.clone(), // use a common genesis block
            None,
            Some(i as u16),
            i == 0, // enable metrics only for the first validator
        )
        .await
        .unwrap();
        validators.push(validator);

        info!("Validator {i} is ready.");
    }

    // Prepare the setup related to the BFT workers.
    let mut tx_clients = validators[0].bft().spawn_tx_clients();

    info!("Preparing a block that will allow the production of transactions.");

    // Initialize the consensus to generate transactions.
    let ledger = test_helpers::CurrentLedger::load(genesis, None).unwrap();
    let consensus = test_helpers::CurrentConsensus::new(ledger, true).unwrap();

    // Use a channel to be able to process transactions as they are created.
    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

    // Generate execution transactions in the background.
    tokio::task::spawn_blocking(move || {
        // TODO (raychu86): Update this bandaid workaround.
        //  Currently the `mint` function can be called without restriction if the recipient is an authorized `beacon`.
        //  Consensus rules will change later when staking and proper coinbase rewards are integrated, which will invalidate this approach.
        //  Note: A more proper way to approach this is to create `split` transactions and then start generating increasingly larger numbers of
        //  transactions, once more and more records are available to you in subsequent blocks.

        // Create inputs for the `credits.aleo/mint` call.
        let inputs = [Value::from_str(&genesis_address.to_string()).unwrap(), Value::from_str("1u64").unwrap()];

        for i in 0.. {
            let transaction = consensus
                .ledger
                .vm()
                .execute(&genesis_private_key, ("credits.aleo", "mint"), inputs.iter(), None, None, &mut rng)
                .unwrap();

            info!("Created transaction {} ({}/inf).", transaction.id(), i + 1);

            tx_sender.send(transaction).unwrap();
        }
    });

    // Note: These transactions do not have conflicting state, so they can be added in any order. However,
    // this means we can't test for conflicts or double spends using these transactions.

    // Create a new test rng for worker and delay randomization (the other one was moved to the transaction
    // creation task). This one doesn't need to be deterministic, it's just fast and readily available.
    let mut rng = TestRng::default();

    // Send the transactions to a random number of BFT workers.
    while let Some(transaction) = tx_receiver.recv().await {
        // Randomize the number of worker recipients.
        let n_recipients: usize = rng.gen_range(1..=4);

        info!("Sending transaction {} to {} workers.", transaction.id(), n_recipients);

        let message = Message::UnconfirmedTransaction(UnconfirmedTransaction {
            transaction_id: transaction.id(),
            transaction: Data::Object(transaction),
        });
        let mut bytes: Vec<u8> = Vec::new();
        message.serialize(&mut bytes).unwrap();
        let payload = bytes::Bytes::from(bytes);
        let tx = TransactionProto { transaction: payload };

        // Submit the transaction to the chosen workers.
        for tx_client in tx_clients.iter_mut().choose_multiple(&mut rng, n_recipients) {
            tx_client.submit_transaction(tx.clone()).await.unwrap();
        }

        // Wait for a random amount of time before processing further transactions.
        let delay: u64 = rng.gen_range(0..2_000);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    // Wait indefinitely.
    std::future::pending::<()>().await;
}
