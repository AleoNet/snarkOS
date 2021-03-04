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
    use snarkos_consensus::Miner;
    use snarkos_testing::consensus::*;
    use snarkvm_algorithms::traits::{
        commitment::CommitmentScheme,
        encryption::EncryptionScheme,
        signature::SignatureScheme,
    };
    use snarkvm_dpc::{AccountAddress, AccountPrivateKey, DPCComponents};
    use snarkvm_objects::{dpc::DPCTransactions, BlockHeader};
    use snarkvm_posw::txids_to_roots;

    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;

    use std::sync::Arc;

    fn keygen<C: DPCComponents, R: Rng>(rng: &mut R) -> (AccountPrivateKey<C>, AccountAddress<C>) {
        let sig_params = C::AccountSignature::setup(rng).unwrap();
        let comm_params = C::AccountCommitment::setup(rng);
        let enc_params = <C::AccountEncryption as EncryptionScheme>::setup(rng);

        let private_key = AccountPrivateKey::<C>::new(&sig_params, &comm_params, rng).unwrap();
        let address = AccountAddress::from_private_key(&sig_params, &comm_params, &enc_params, &private_key).unwrap();

        (private_key, address)
    }

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    fn test_find_block(transactions: &DPCTransactions<TestTx>, parent_header: &BlockHeader) {
        let consensus = Arc::new(TEST_CONSENSUS.clone());
        let mut rng = XorShiftRng::seed_from_u64(3); // use this rng so that a valid solution is found quickly

        let (_, miner_address) = keygen(&mut rng);
        let miner = Miner::new(miner_address, consensus.clone());

        let header = miner.find_block(transactions, parent_header).unwrap();

        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transactions.to_transaction_ids().unwrap());

        // ensure that our POSW proof passes
        consensus
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[test]
    fn find_valid_block() {
        let transactions = DPCTransactions(vec![TestTx; 3]);
        let parent_header = genesis().header;
        test_find_block(&transactions, &parent_header);
    }
}
