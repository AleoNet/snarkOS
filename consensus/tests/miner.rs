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

mod miner {
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };

    use futures::executor::block_on;
    use snarkos_consensus::{error::ConsensusError, MineContext};
    use snarkos_storage::{SerialBlockHeader, SerialTransaction};
    use snarkos_testing::{sync::*, wait_until};
    use snarkvm_algorithms::{
        traits::{commitment::CommitmentScheme, encryption::EncryptionScheme, signature::SignatureScheme},
        SNARKError,
    };
    use snarkvm_dpc::{Address, DPCComponents, PrivateKey};
    use snarkvm_posw::{error::PoswError, txids_to_roots};

    use rand::{CryptoRng, Rng, SeedableRng};
    use rand_chacha::ChaChaRng;
    use tokio::sync::mpsc;

    fn keygen<C: DPCComponents, R: Rng + CryptoRng>(rng: &mut R) -> (PrivateKey<C>, Address<C>) {
        let sig_params = C::AccountSignature::setup(rng).unwrap();
        let comm_params = C::AccountCommitment::setup(rng);
        let enc_params = <C::AccountEncryption as EncryptionScheme>::setup(rng);

        let private_key = PrivateKey::<C>::new(&sig_params, &comm_params, rng).unwrap();
        let address = Address::from_private_key(&sig_params, &comm_params, &enc_params, &private_key).unwrap();

        (private_key, address)
    }

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    async fn test_find_block(transactions: &[SerialTransaction], parent_header: &SerialBlockHeader) {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let mut rng = ChaChaRng::seed_from_u64(3); // use this rng so that a valid solution is found quickly

        let (_, miner_address) = keygen(&mut rng);
        let miner = MineContext::prepare(miner_address, consensus.clone()).await.unwrap();

        let header = miner
            .find_block(transactions, parent_header, &AtomicBool::new(false))
            .unwrap();

        let transaction_ids = transactions.iter().map(|x| x.id).collect::<Vec<_>>();
        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transaction_ids[..]);

        // ensure that our POSW proof passes
        consensus
            .parameters
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[tokio::test]
    async fn find_valid_block() {
        let transactions = vec![
            DATA.block_1.transactions[0].clone(),
            DATA.block_2.transactions[0].clone(),
        ];
        let parent_header = genesis().header.into();
        test_find_block(&transactions, &parent_header).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn terminate_on_block() {
        tracing_subscriber::fmt::init();
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        consensus.fetch_memory_pool().await; // just make sure we're fully initialized by blocking on consensus call

        let miner_address = FIXTURE_VK.test_accounts[0].address.clone();
        let miner = MineContext::prepare(miner_address, consensus.clone()).await.unwrap();

        let (sender, mut receiver) = mpsc::channel(10);

        let terminator = Arc::new(AtomicBool::new(false));
        let terminator_clone = terminator.clone();
        tokio::task::spawn_blocking(move || {
            let sender_clone = sender.clone();
            let mining = async move {
                let candidate_transactions = miner.consensus.fetch_memory_pool().await;
                println!("creating a block");

                let (transactions, _coinbase_records) = miner.establish_block(candidate_transactions).await?;

                println!("generated a coinbase transaction");
                sender_clone.try_send("started").ok().unwrap();

                miner.find_block(&transactions, &genesis().header.into(), &terminator_clone)?;

                Ok(())
            };
            match block_on(mining) {
                Err(ConsensusError::PoswError(PoswError::SnarkError(SNARKError::Terminated))) => {
                    sender.try_send("terminated").ok().unwrap();
                }
                Err(e) => panic!("block mining failed for bad reason: {:?}", e),
                Ok(_) => panic!("block mining passed and shouldn't have"),
            };
        });
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(60), receiver.recv())
                .await
                .unwrap(),
            Some("started")
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        terminator.store(true, Ordering::SeqCst);
        wait_until!(60, !terminator.load(Ordering::SeqCst));
        assert_eq!(receiver.recv().await.unwrap(), "terminated");
    }
}
