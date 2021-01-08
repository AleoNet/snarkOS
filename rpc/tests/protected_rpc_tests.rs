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
    use snarkos_network::Server;
    use snarkos_rpc::*;
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::*};
    use snarkvm_dpc::base_dpc::{
        instantiated::{Components, Tx},
        parameters::PublicParameters,
        record::DPCRecord,
    };
    use snarkvm_models::dpc::Record;
    use snarkvm_objects::{AccountAddress, AccountPrivateKey, AccountViewKey};
    use snarkvm_utilities::{
        bytes::{FromBytes, ToBytes},
        to_bytes,
    };

    use jsonrpc_core::MetaIoHandler;
    use parking_lot::{Mutex, RwLock};
    use serde_json::Value;
    use std::{str::FromStr, sync::Arc};

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

    fn unwrap_arc_rwlock<T>(x: Arc<RwLock<T>>) -> T {
        if let Ok(lock) = Arc::try_unwrap(x) {
            lock.into_inner()
        } else {
            panic!("can't unwrap the Arc, there are strong refs left!");
        }
    }

    async fn initialize_test_rpc(
        storage: Arc<RwLock<MerkleTreeLedger>>,
        parameters: PublicParameters<Components>,
    ) -> MetaIoHandler<Meta> {
        let consensus = TEST_CONSENSUS.clone();

        let credentials = RpcCredentials {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        };

        let memory_pool = Arc::new(Mutex::new(MemoryPool::new()));

        let environment = initialize_test_environment(None, vec![], storage.clone(), parameters.clone()).unwrap();
        let server = Server::new(environment.clone()).await.unwrap();

        let storage_path = storage.read().storage.db.path().to_path_buf();

        let rpc_impl = RpcImpl::new(
            storage,
            storage_path,
            parameters,
            environment,
            consensus,
            memory_pool,
            Some(credentials),
            server,
        );
        let mut io = jsonrpc_core::MetaIoHandler::default();

        rpc_impl.add_protected(&mut io);

        io
    }

    #[tokio::test]
    async fn test_rpc_authentication() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = invalid_authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String("Authentication Error".to_string());
        assert_eq!(extracted["error"]["message"], expected_result);

        drop(rpc);
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitment_count() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

        storage.write().store_record(&DATA.records_1[0]).unwrap();

        let method = "getrecordcommitmentcount".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(extracted["result"], 1);

        drop(rpc);
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitments() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

        storage.write().store_record(&DATA.records_1[0]).unwrap();

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::Array(vec![Value::String(hex::encode(
            to_bytes![DATA.records_1[0].commitment()].unwrap(),
        ))]);

        assert_eq!(extracted["result"], expected_result);

        drop(rpc);
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_get_raw_record() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

        storage.write().store_record(&DATA.records_1[0]).unwrap();

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
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_decode_record() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

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
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_decrypt_record() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

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
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_rpc_create_raw_transaction() {
        let storage = Arc::new(RwLock::new(FIXTURE.ledger()));
        let parameters = FIXTURE.parameters.clone();
        let meta = authentication();

        let consensus = TEST_CONSENSUS.clone();

        consensus
            .receive_block(&parameters, &storage.read(), &mut MemoryPool::new(), &DATA.block_1)
            .unwrap();

        let io = initialize_test_rpc(storage.clone(), parameters).await;

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
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }

    #[tokio::test]
    async fn test_create_account() {
        let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(storage.clone(), parameters).await;

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
        kill_storage_sync(unwrap_arc_rwlock(storage));
    }
}
