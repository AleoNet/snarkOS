mod rpc_tests {
    use snarkos_consensus::get_block_reward;
    use snarkos_dpc::dpc::base_dpc::instantiated::{MerkleTreeLedger, Tx};
    use snarkos_models::objects::Transaction;
    use snarkos_rpc::*;
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::*};
    use snarkos_utilities::{
        bytes::{FromBytes, ToBytes},
        to_bytes,
    };

    use jsonrpc_test as json_test;
    use jsonrpc_test::Rpc;
    use serde_json::Value;
    use snarkos_models::dpc::Record;
    use std::{net::SocketAddr, sync::Arc};

    fn initialize_test_rpc(storage: &Arc<MerkleTreeLedger>) -> Rpc {
        let bootnode_address = random_socket_address();
        let server_address = random_socket_address();

        let parameters = load_verifying_parameters();

        let server = initialize_test_server(
            server_address,
            bootnode_address,
            storage.clone(),
            parameters,
            CONNECTION_FREQUENCY_LONG,
        );

        let consensus = TEST_CONSENSUS.clone();
        json_test::Rpc::new(
            RpcImpl::new(
                storage.clone(),
                server.context.clone(),
                consensus,
                server.memory_pool_lock,
            )
            .to_delegate(),
        )
    }

    fn verify_transaction_info(transaction_bytes: Vec<u8>, transaction_info: Value) {
        let transaction = Tx::read(&transaction_bytes[..]).unwrap();

        let transaction_id = hex::encode(transaction.transaction_id().unwrap());
        let transaction_size = transaction_bytes.len();
        let old_serial_numbers: Vec<Value> = transaction
            .old_serial_numbers()
            .iter()
            .map(|sn| Value::String(hex::encode(to_bytes![sn].unwrap())))
            .collect();
        let new_commitments: Vec<Value> = transaction
            .new_commitments()
            .iter()
            .map(|cm| Value::String(hex::encode(to_bytes![cm].unwrap())))
            .collect();
        let memo = hex::encode(transaction.memorandum());

        assert_eq!(transaction_id, transaction_info["txid"]);
        assert_eq!(transaction_size, transaction_info["size"]);
        assert_eq!(Value::Array(old_serial_numbers), transaction_info["old_serial_numbers"]);
        assert_eq!(Value::Array(new_commitments), transaction_info["new_commitments"]);
        assert_eq!(memo, transaction_info["memo"]);

        let digest = hex::encode(to_bytes![transaction.digest].unwrap());
        let transaction_proof = hex::encode(to_bytes![transaction.transaction_proof].unwrap());
        let predicate_commitment = hex::encode(to_bytes![transaction.predicate_commitment].unwrap());
        let local_data_commitment = hex::encode(to_bytes![transaction.local_data_commitment].unwrap());
        let value_balance = transaction.value_balance;
        let signatures: Vec<Value> = transaction
            .signatures
            .iter()
            .map(|s| Value::String(hex::encode(to_bytes![s].unwrap())))
            .collect();

        assert_eq!(digest, transaction_info["digest"]);
        assert_eq!(transaction_proof, transaction_info["transaction_proof"]);
        assert_eq!(predicate_commitment, transaction_info["predicate_commitment"]);
        assert_eq!(local_data_commitment, transaction_info["local_data_commitment"]);
        assert_eq!(value_balance, transaction_info["value_balance"]);
        assert_eq!(Value::Array(signatures), transaction_info["signatures"]);
    }

    fn make_request_no_params(rpc: &Rpc, method: String) -> Value {
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method,);

        let response = rpc.io.handle_request_sync(&request).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        extracted["result"].clone()
    }

    #[test]
    fn test_get_block_call() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let response = rpc.request("getblock", &[hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())]);

        let block_response: Value = serde_json::from_str(&response).unwrap();

        let genesis_block = genesis();

        assert_eq!(hex::encode(genesis_block.header.get_hash().0), block_response["hash"]);
        assert_eq!(
            hex::encode(genesis_block.header.merkle_root_hash.0),
            block_response["merkle_root"]
        );
        assert_eq!(
            hex::encode(genesis_block.header.previous_block_hash.0),
            block_response["previous_block_hash"]
        );
        assert_eq!(genesis_block.header.time, block_response["time"]);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_block_count() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let method = "getblockcount".to_string();

        let result = make_request_no_params(&rpc, method);

        assert_eq!(result.as_u64().unwrap(), 1u64);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_best_block_hash() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let method = "getbestblockhash".to_string();

        let result = make_request_no_params(&rpc, method);

        assert_eq!(
            result.as_str().unwrap(),
            hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())
        );

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_block_hash() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        assert_eq!(rpc.request("getblockhash", &[0u32]), format![
            r#""{}""#,
            hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())
        ]);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_raw_transaction() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let genesis_block = genesis();

        let transaction = &genesis_block.transactions.0[0];
        let transaction_id = hex::encode(transaction.transaction_id().unwrap());

        assert_eq!(rpc.request("getrawtransaction", &[transaction_id]), format![
            r#""{}""#,
            hex::encode(to_bytes![transaction].unwrap())
        ]);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_transaction_info() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let genesis_block = genesis();
        let transaction = &genesis_block.transactions.0[0];

        let response = rpc.request("gettransactioninfo", &[hex::encode(
            transaction.transaction_id().unwrap(),
        )]);

        let transaction_info: Value = serde_json::from_str(&response).unwrap();

        verify_transaction_info(to_bytes![transaction].unwrap(), transaction_info);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_decode_raw_transaction() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let response = rpc.request("decoderawtransaction", &[hex::encode(TRANSACTION_1.to_vec())]);

        let transaction_info: Value = serde_json::from_str(&response).unwrap();

        verify_transaction_info(TRANSACTION_1.to_vec(), transaction_info);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_send_raw_transaction() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();

        assert_eq!(
            rpc.request("sendtransaction", &[hex::encode(TRANSACTION_1.to_vec())]),
            format![r#""{}""#, hex::encode(transaction.transaction_id().unwrap())]
        );

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_decode_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let record = &DATA.records_1[0];

        let response = rpc.request("decoderecord", &[hex::encode(to_bytes![record].unwrap())]);
        let record_info: Value = serde_json::from_str(&response).unwrap();

        let account_public_key = hex::encode(to_bytes![record.account_public_key()].unwrap());
        let is_dummy = record.is_dummy();
        let value = record.value();
        let birth_predicate_repr = hex::encode(to_bytes![record.birth_predicate_repr()].unwrap());
        let death_predicate_repr = hex::encode(to_bytes![record.death_predicate_repr()].unwrap());
        let serial_number_nonce = hex::encode(to_bytes![record.serial_number_nonce()].unwrap());
        let commitment = hex::encode(to_bytes![record.commitment()].unwrap());
        let commitment_randomness = hex::encode(to_bytes![record.commitment_randomness()].unwrap());

        assert_eq!(account_public_key, record_info["account_public_key"]);
        assert_eq!(is_dummy, record_info["is_dummy"]);
        assert_eq!(value, record_info["value"]);
        assert_eq!(birth_predicate_repr, record_info["birth_predicate_repr"]);
        assert_eq!(death_predicate_repr, record_info["death_predicate_repr"]);
        assert_eq!(serial_number_nonce, record_info["serial_number_nonce"]);
        assert_eq!(commitment, record_info["commitment"]);
        assert_eq!(commitment_randomness, record_info["commitment_randomness"]);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_connection_count() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let method = "getconnectioncount".to_string();

        let result = make_request_no_params(&rpc, method);

        assert_eq!(result.as_u64().unwrap(), 0u64);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_peer_info() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let method = "getpeerinfo".to_string();

        let result = make_request_no_params(&rpc, method);

        let peer_info: PeerInfo = serde_json::from_value(result).unwrap();

        let expected_peers: Vec<SocketAddr> = vec![];

        assert_eq!(peer_info.peers, expected_peers);

        drop(rpc);
        kill_storage_sync(storage);
    }

    #[test]
    fn test_get_block_template() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let rpc = initialize_test_rpc(&storage);

        let method = "getblocktemplate".to_string();

        let result = make_request_no_params(&rpc, method);

        let template: BlockTemplate = serde_json::from_value(result).unwrap();

        let expected_transactions: Vec<String> = vec![];

        let new_height = storage.get_latest_block_height() + 1;
        let block_reward = get_block_reward(new_height);
        let latest_block_hash = hex::encode(storage.get_latest_block().unwrap().header.get_hash().0);

        assert_eq!(template.previous_block_hash, latest_block_hash);
        assert_eq!(template.block_height, new_height);
        assert_eq!(template.transactions, expected_transactions);
        assert!(template.coinbase_value >= block_reward);

        drop(rpc);
        kill_storage_sync(storage);
    }
}
