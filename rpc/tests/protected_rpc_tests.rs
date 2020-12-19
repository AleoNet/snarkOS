// Copyright (C) 2019-2020 Aleo Systems Inc.
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

/// Tests for protected RPC endpoints
mod protected_rpc_tests {
    use snarkos_consensus::{memory_pool::MemoryPool, MerkleTreeLedger};
    use snarkos_dpc::base_dpc::{
        instantiated::{Components, Tx},
        parameters::PublicParameters,
        record::DPCRecord,
        TransactionKernel,
    };
    use snarkos_models::dpc::Record;
    use snarkos_network::{external::SyncHandler, internal::context::Context};
    use snarkos_objects::{AccountAddress, AccountPrivateKey, AccountViewKey};
    use snarkos_rpc::*;
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::*};
    use snarkos_utilities::{
        bytes::{FromBytes, ToBytes},
        to_bytes,
    };

    use jsonrpc_core::MetaIoHandler;
    use serde_json::Value;
    use std::{str::FromStr, sync::Arc};
    use tokio::sync::Mutex;

    const TEST_USERNAME: &str = "TEST_USERNAME";
    const TEST_PASSWORD: &str = "TEST_PASSWORD";

    fn invalid_authentication() -> Meta {
        let basic_auth_encoding = format!(
            "Basic {}",
            base64::encode(format!("{}:{}", "INVALID_USERNAME", "INVALID_PASSWORD"))
        );

        Meta {
            auth: Some(basic_auth_encoding),
        }
    }

    fn authentication() -> Meta {
        let basic_auth_encoding = format!(
            "Basic {}",
            base64::encode(format!("{}:{}", TEST_USERNAME, TEST_PASSWORD))
        );

        Meta {
            auth: Some(basic_auth_encoding),
        }
    }

    fn initialize_test_rpc(
        storage: &Arc<MerkleTreeLedger>,
        parameters: PublicParameters<Components>,
    ) -> MetaIoHandler<Meta> {
        let server_address = random_socket_address();
        let consensus = TEST_CONSENSUS.clone();

        let credentials = RpcCredentials {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        };

        let memory_pool = MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

        let sync_handler = SyncHandler::new(server_address);
        let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

        let context = Context::new(server_address, 5, 1, 10, true, vec![], false);

        let storage = storage.clone();
        let storage_path = storage.storage.db.path().to_path_buf();

        let rpc_impl = RpcImpl::new(
            storage,
            storage_path,
            parameters,
            Arc::new(context),
            consensus,
            memory_pool_lock,
            sync_handler_lock,
            Some(credentials),
        );
        let mut io = jsonrpc_core::MetaIoHandler::default();

        rpc_impl.add_protected(&mut io);

        io
    }

    #[test]
    fn test_rpc_authentication() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = invalid_authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String("Authentication Error".to_string());
        assert_eq!(extracted["error"]["message"], expected_result);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_fetch_record_commitment_count() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        storage.store_record(&DATA.records_1[0]).unwrap();

        let method = "getrecordcommitmentcount".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(extracted["result"], 1);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_fetch_record_commitments() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        storage.store_record(&DATA.records_1[0]).unwrap();

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::Array(vec![Value::String(hex::encode(
            to_bytes![DATA.records_1[0].commitment()].unwrap(),
        ))]);

        assert_eq!(extracted["result"], expected_result);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_get_raw_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        storage.store_record(&DATA.records_1[0]).unwrap();

        let method = "getrawrecord".to_string();
        let params = hex::encode(to_bytes![DATA.records_1[0].commitment()].unwrap());
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, params
        );
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String(hex::encode(to_bytes![DATA.records_1[0]].unwrap()));

        assert_eq!(extracted["result"], expected_result);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_decode_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        let record = &DATA.records_1[0];

        let method = "decoderecord";
        let params = hex::encode(to_bytes![record].unwrap());
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, params
        );

        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let record_info: Value = serde_json::from_str(&response).unwrap();

        let record_info = record_info["result"].clone();

        let owner = record.owner().to_string();
        let is_dummy = record.is_dummy();
        let value = record.value();
        let birth_program_id = hex::encode(to_bytes![record.birth_program_id()].unwrap());
        let death_program_id = hex::encode(to_bytes![record.death_program_id()].unwrap());
        let serial_number_nonce = hex::encode(to_bytes![record.serial_number_nonce()].unwrap());
        let commitment = hex::encode(to_bytes![record.commitment()].unwrap());
        let commitment_randomness = hex::encode(to_bytes![record.commitment_randomness()].unwrap());

        assert_eq!(owner, record_info["owner"]);
        assert_eq!(is_dummy, record_info["is_dummy"]);
        assert_eq!(value, record_info["value"]);
        assert_eq!(birth_program_id, record_info["birth_program_id"]);
        assert_eq!(death_program_id, record_info["death_program_id"]);
        assert_eq!(serial_number_nonce, record_info["serial_number_nonce"]);
        assert_eq!(commitment, record_info["commitment"]);
        assert_eq!(commitment_randomness, record_info["commitment_randomness"]);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_decrypt_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        let system_parameters = &FIXTURE_VK.parameters.system_parameters;
        let [miner_acc, _, _] = FIXTURE_VK.test_accounts.clone();

        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        let ciphertexts = transaction.encrypted_records;

        let records = &DATA.records_1;

        let view_key = AccountViewKey::from_private_key(
            &system_parameters.account_signature,
            &system_parameters.account_commitment,
            &miner_acc.private_key,
        )
        .unwrap();

        for (ciphertext, record) in ciphertexts.iter().zip(records) {
            let ciphertext_string = hex::encode(to_bytes![ciphertext].unwrap());
            let account_view_key = view_key.to_string();

            let params = DecryptRecordInput {
                encrypted_record: ciphertext_string,
                account_view_key,
            };
            let params = serde_json::to_value(params).unwrap();

            let method = "decryptrecord";
            let request = format!(
                "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}] }}",
                method, params
            );
            let response = rpc.handle_request_sync(&request, meta.clone()).unwrap();

            let extracted: Value = serde_json::from_str(&response).unwrap();

            let expected_result = Value::String(hex::encode(to_bytes![record].unwrap()).to_string());
            assert_eq!(extracted["result"], expected_result);
        }

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_create_raw_transaction() {
        let storage = Arc::new(FIXTURE.ledger());
        let parameters = FIXTURE.parameters.clone();
        let meta = authentication();

        let consensus = TEST_CONSENSUS.clone();

        consensus
            .receive_block(&parameters, &storage, &mut MemoryPool::new(), &DATA.block_1)
            .unwrap();

        let io = initialize_test_rpc(&storage, parameters);

        let method = "createrawtransaction".to_string();

        let [sender, receiver, _] = &FIXTURE_VK.test_accounts;

        let old_records = vec![hex::encode(to_bytes![DATA.records_1[0]].unwrap())];
        let old_account_private_keys = vec![sender.private_key.to_string()];

        let recipients = vec![TransactionRecipient {
            address: receiver.address.to_string(),
            amount: 100,
        }];

        let network_id = 0;

        let params = TransactionInputs {
            old_records,
            old_account_private_keys,
            recipients,
            memo: None,
            network_id,
        };

        let params = serde_json::to_value(params).unwrap();
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}] }}",
            method, params
        );
        let response = io.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Components> = FromBytes::read(&record_bytes[..]).unwrap();
        }

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Tx = FromBytes::read(&transaction_bytes[..]).unwrap();

        drop(io);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_create_transaction_kernel() {
        let storage = Arc::new(FIXTURE.ledger());
        let parameters = FIXTURE.parameters.clone();
        let meta = authentication();

        let consensus = TEST_CONSENSUS.clone();

        consensus
            .receive_block(&parameters, &storage, &mut MemoryPool::new(), &DATA.block_1)
            .unwrap();

        let io = initialize_test_rpc(&storage, parameters);

        let method = "createtransactionkernel".to_string();

        let [sender, receiver, _] = &FIXTURE_VK.test_accounts;

        let old_records = vec![hex::encode(to_bytes![DATA.records_1[0]].unwrap())];
        let old_account_private_keys = vec![sender.private_key.to_string()];

        let recipients = vec![TransactionRecipient {
            address: receiver.address.to_string(),
            amount: 100,
        }];

        let network_id = 0;

        let params = TransactionInputs {
            old_records,
            old_account_private_keys,
            recipients,
            memo: None,
            network_id,
        };

        let params = serde_json::to_value(params).unwrap();
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}] }}",
            method, params
        );
        let response = io.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        let transaction_kernel_bytes = hex::decode(result.as_str().unwrap()).unwrap();
        let _transaction_kernel: TransactionKernel<Components> =
            FromBytes::read(&transaction_kernel_bytes[..]).unwrap();

        drop(io);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_rpc_create_transaction_1() {
        let storage = Arc::new(FIXTURE.ledger());
        let parameters = FIXTURE.parameters.clone();
        let meta = authentication();

        let consensus = TEST_CONSENSUS.clone();

        consensus
            .receive_block(&parameters, &storage, &mut MemoryPool::new(), &DATA.block_1)
            .unwrap();

        let io = initialize_test_rpc(&storage, parameters);

        let method = "createtransaction".to_string();

        // TODO (raychu86): Generate the transaction kernel with the test data.
        let transaction_kernel = "c47a5e10158beb772e62ff52bfc6c4ab9ff554af3e5201ab50dea2e1de5529e9c47a5e10158beb772e62ff52bfc6c4ab9ff554af3e5201ab50dea2e1de5529e9ae7c26acc4e65698fc71ec04d6b55554e920d55a71903396b5f395f2cf4242010080d1f008000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00d0cb5667bcd6d6f1c82c997e900549632a0e411cf83e396953dcad27b4cdf105b8e07d3830a65bb0f160cd1c04c411abdb130ba71c2a31e7b4e853b57923190bbd60a387eafb1e5bbe5c734ad77b9ea41ff7181d2f507e6af54eac782e844d02ae7c26acc4e65698fc71ec04d6b55554e920d55a71903396b5f395f2cf4242010100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00766e26376e8ec1fc3b54d7ee9c8ee4f4e635be883f4074c48c4315fad41a3008c645e0fed6be4620434c5ae450fb37f813f54e6886485a510704c28a1768550872046ee6921fcb355570aa8fac017845018b4f3f93f31f1627033df0c275290248c11ff2a55deabbd980c41703cd52217c290e70a56cfe2f64c19db4b87a2702e6476fd0b35601def2831bfbcb2a34bbe542e0a415f33d2ee318c408ea536908339215a763ebfeeb3f5aec8785e6e0db77f20ab99b5f4f1f29f2c45be721400c96d9171a7c9167e357a9dd3ab749a0df576e2bbac6876db5fcbf493e612aaf0a203d0dcb32884ef33c68dbd476a5d2cfd9b073ff85aa3141b8e8df2d985431fcc5209fc5f51558e71318ca61663e73efbe9420ad3bbda2ed45cc0da6adfef65ea861f5065e80dc1cfe0ebb99dd4e08b538fba5dafa4f5e41f6aff7cf62f3fe8ca9120064000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00a0e854f9f2d948e7b7a53977a2cf957ebd4e22de9945838f3336c91a91368f12d989399911d329380afad2d97edc6fcb0fa1edd5c3fbeb793316f892bda8690eecdbe54506c333bdc4ecedf73db0c10775c1f152ce8ce47c829f950817da6703f5065e80dc1cfe0ebb99dd4e08b538fba5dafa4f5e41f6aff7cf62f3fe8ca9120100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00994dfda5be542c78e321822194d9f8a0f5ecf1091e7864d9f29e40dea8102b056e85a21342dbff9d753555f0d62f7f67d8d74060d336e9ffb3b9c0c04a0a8603f2d71325d46bef53b4da3f235c3086410e8b59f1e409a81ab472ddb0e86b1902c019ff0107e1e57d330ea46fbad17ea2e7fe45b7f75e12be446e51446a23e06720fee3bbf9bba6506f90dac3b7aaf0f3ee5bf650e66f20ccaf4e325b15342fb6d989399911d329380afad2d97edc6fcb0fa1edd5c3fbeb793316f892bda8690e6e85a21342dbff9d753555f0d62f7f67d8d74060d336e9ffb3b9c0c04a0a860322aa5d0fbe1ea1c679f8130b2739befa520140d96b3ced844dbb08a32d913c013910a63eeca2aade76ac7556177c12b14cf9bb4b1709659db095565de0ae1d03086828b731b17746460c7a3028ff4a744bd275bd398cb24681432a7e3080b8cb041835cfb5fff06ee9bf499ed47b51477b0bb7b402f82605b005f702cebc599c0ba009fe2a48123f670b5df9813ec896b22f9b88ba0ddf7ad6ef4337f439154501f482b4e36fbe059fa43990dc8c9b00a9ca9ea1f8516695a61db8b797d2d2a30f538d0f9451f050678d60f884baf9dd54659cc3d7a44c580941bc69a95aab6b0615d3c8a19a3a80f487af0a84e89b93836cf07332e22796d13e2cda34599cfa04b190cc871b4f63379706991dbbb6faed1e173266c0218955d56e23e56bd0a1027ec5acdebfd062f6752b5c03904387b7df9b28dd3b54266c475fa37528a02705e800089dc0a367d38e962538ea8b7848be1856178bff3ca2d36a0211b80d6de3fbc9029ab2c48c451db5d16a0512c1a9a72b3c09a15e3fd2fe49e68571267fb5b1e805681b92d2eace67211b4e774ef6e489cf5f901f290b3ebb12c836331c76e05f0c80f4e883bd69fde147631511558c210e801372207e03ae50c2552b0c8946960eed20690fcd18be7305544ad411bfefe5303776358bf35682d734cbc3a4818e013fa2887915477720fbb6a7135e09fab565885ce134b186f17b76790c307a6d10454fbc8d186cd47919d04a29dbf107f9d8c7e66e4d5b4fd1d05bc11235a35108b99061999404458d00e24918ba00940676852f5a16ccef62612b346e5e7da1061500c9074f39c90f39e64fc143dd3f24d83393e8034ee89187c6897a5b06c02a970821282d903fd45fbb0fec18a4ee1fbbd381ce343ee837195cace0fe582c2e7d00759407f22dc2a1b7549d3d599b522f82f25a91fe143158e09a2cae2da3b4c8d139b13d0d4619e6375f8e8ba34fe98f3dd47c3c4653c481e0b4d6e24c86a90ef9ffa71452bfa9e2d2895274e0f9ec2f4b6864efcd8ea2ad6e3d93513d6bdbd003303b78bf5acbf39078741643ff820963e3d07622c1a1dfd7d119e7afa57a910810934041214109307bc2521037a24526dcc8fafefaee23913c76b6e2ca86fa117f92506ea4626bc50bf4aa4ad0517d9d082583a0f081ccab7dd814e3ae0ff60dd095cee83c1fab048ea978294bad002fc567d4035ac0597943096feb9f84e0061e47c3ddcb2b945e1ba6eccb15c4582e17ee335ccaaa4e59c6d3a0f9a3c4560f54421424c1b13abd3f7fc7bb65b25aa9d22a0bce97fef92388ac76c730550508d4d49bf5e0cee202a0d22160c6076797e0b637ea135dad54453a22863fb35b03555546e4954933c990a90cd915b5af8b0d47964d591dcdbc17da8c1b15d29303b6340e7c6a23d630e3b360be0b669082042e620dc8fef36571a885057e8b1c00f6ee607a3a1657d12de246e7e96895fd3d44424348f9146a34b39e6add291e001cd1f008000000008693d11586cdeee4a51278499481ea4dd725705222f02b2a7a4dd82a9ad437e100";

        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, transaction_kernel
        );
        let response = io.handle_request_sync(&request, meta).unwrap();

        println!("extracted: {}", response);

        let extracted: Value = serde_json::from_str(&response).unwrap();

        println!("extracted: {}", extracted);

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Components> = FromBytes::read(&record_bytes[..]).unwrap();
        }

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Tx = FromBytes::read(&transaction_bytes[..]).unwrap();

        drop(io);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_create_account() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        let method = "createaccount".to_string();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta.clone()).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = AccountPrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = AccountAddress::<Components>::from_str(&account.address).unwrap();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = AccountPrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = AccountAddress::<Components>::from_str(&account.address).unwrap();

        drop(rpc);
        kill_storage_sync(storage);
    }
}
