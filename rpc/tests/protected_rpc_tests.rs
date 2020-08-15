/// Tests for protected RPC endpoints
mod protected_rpc_tests {
    use snarkos_consensus::{memory_pool::MemoryPool, MerkleTreeLedger};
    use snarkos_dpc::base_dpc::{
        instantiated::{Components, Tx},
        parameters::PublicParameters,
        record::DPCRecord,
    };
    use snarkos_models::dpc::Record;
    use snarkos_network::Context;
    use snarkos_objects::{AccountAddress, AccountPrivateKey};
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

        let context = Context::new(server_address, 5, 1, 10, true, vec![]);

        let rpc_impl = RpcImpl::new(
            storage.clone(),
            parameters,
            Arc::new(context),
            consensus,
            memory_pool_lock,
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
    fn test_create_account() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let parameters = load_verifying_parameters();
        let meta = authentication();
        let rpc = initialize_test_rpc(&storage, parameters);

        let method = "createaccount".to_string();

        // Request without specified metadata
        let request_without_metadata = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc
            .handle_request_sync(&request_without_metadata, meta.clone())
            .unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = AccountPrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = AccountAddress::<Components>::from_str(&account.address).unwrap();

        // Request without specified metadata
        let request_without_metadata = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request_without_metadata, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = AccountPrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = AccountAddress::<Components>::from_str(&account.address).unwrap();

        drop(rpc);
        kill_storage_sync(storage);
    }
}
