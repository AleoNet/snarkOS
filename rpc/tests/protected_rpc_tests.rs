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

/// Tests for protected RPC endpoints
mod protected_rpc_tests {
    use snarkos_consensus::{Consensus, MerkleTreeLedger};
    use snarkos_network::Node;
    use snarkos_rpc::*;
    use snarkos_storage::LedgerStorage;
    use snarkos_testing::{
        network::{test_config, ConsensusSetup, TestSetup},
        sync::*,
    };

    use snarkvm::{
        dpc::{
            testnet1::{Testnet1Parameters, Testnet1Transaction},
            Address,
            PrivateKey,
            RecordScheme,
            TransactionAuthorization,
            ViewKey,
        },
        utilities::{to_bytes_le, FromBytes, ToBytes},
    };

    use jsonrpc_core::MetaIoHandler;
    use serde_json::Value;
    use std::{str::FromStr, sync::Arc, time::Duration};

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

    async fn initialize_test_rpc(
        ledger: Arc<MerkleTreeLedger<LedgerStorage>>,
    ) -> (MetaIoHandler<Meta>, Arc<Consensus<LedgerStorage>>) {
        let credentials = RpcCredentials {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        };

        let environment = test_config(TestSetup::default());
        let mut node = Node::new(environment).unwrap();
        let consensus_setup = ConsensusSetup::default();
        let consensus = Arc::new(snarkos_testing::sync::create_test_consensus_from_ledger(ledger.clone()));

        let node_consensus = snarkos_network::Sync::new(
            consensus.clone(),
            consensus_setup.is_miner,
            Duration::from_secs(consensus_setup.block_sync_interval),
            Duration::from_secs(consensus_setup.tx_sync_interval),
        );

        node.set_sync(node_consensus);

        let rpc_impl = RpcImpl::new(ledger, Some(credentials), node);
        let mut io = jsonrpc_core::MetaIoHandler::default();

        rpc_impl.add_protected(&mut io);

        (io, consensus)
    }

    #[tokio::test]
    async fn test_rpc_authentication() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let meta = invalid_authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String("Authentication Error".to_string());
        assert_eq!(extracted["error"]["message"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitment_count() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        storage.store_record(&DATA.records_1[0]).unwrap();

        let meta = authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let method = "getrecordcommitmentcount".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(extracted["result"], 1);
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitments() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        storage.store_record(&DATA.records_1[0]).unwrap();

        let meta = authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::Array(vec![Value::String(hex::encode(
            to_bytes_le![DATA.records_1[0].commitment()].unwrap(),
        ))]);

        assert_eq!(extracted["result"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_get_raw_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        storage.store_record(&DATA.records_1[0]).unwrap();

        let meta = authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let method = "getrawrecord".to_string();
        let params = hex::encode(to_bytes_le![DATA.records_1[0].commitment()].unwrap());
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, params
        );
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String(hex::encode(to_bytes_le![DATA.records_1[0]].unwrap()));

        assert_eq!(extracted["result"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_decrypt_record() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let meta = authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let [miner_acc, _, _] = FIXTURE_VK.test_accounts.clone();

        let transaction = Testnet1Transaction::read_le(&TRANSACTION_1[..]).unwrap();
        let ciphertexts = transaction.encrypted_records;

        let records = &DATA.records_1;

        let view_key = ViewKey::from_private_key(&miner_acc.private_key).unwrap();

        for (ciphertext, record) in ciphertexts.iter().zip(records) {
            let ciphertext_string = hex::encode(to_bytes_le![ciphertext].unwrap());
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

            let expected_result = Value::String(hex::encode(to_bytes_le![record].unwrap()).to_string());
            assert_eq!(extracted["result"], expected_result);
        }
    }

    #[tokio::test]
    async fn test_rpc_create_raw_transaction() {
        let storage = Arc::new(FIXTURE.ledger());
        let meta = authentication();

        let (rpc, consensus) = initialize_test_rpc(storage).await;

        consensus.receive_block(&DATA.block_1, false).await.unwrap();

        let method = "createrawtransaction".to_string();

        let [sender, receiver, _] = &FIXTURE_VK.test_accounts;

        let old_records = vec![hex::encode(to_bytes_le![DATA.records_1[0]].unwrap())];
        let old_account_private_keys = vec![sender.private_key.to_string()];

        let recipients = vec![TransactionRecipient {
            address: receiver.address.to_string(),
            amount: 100,
        }];

        let network_id = 1;

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
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Testnet1Transaction = FromBytes::read_le(&transaction_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_rpc_create_transaction_authorization() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let meta = authentication();

        let (rpc, consensus) = initialize_test_rpc(storage).await;

        consensus.receive_block(&DATA.block_1, false).await.unwrap();

        let method = "createtransactionauthorization".to_string();

        let [sender, receiver, _] = &FIXTURE_VK.test_accounts;

        let old_records = vec![hex::encode(to_bytes_le![DATA.records_1[0]].unwrap())];
        let old_account_private_keys = vec![sender.private_key.to_string()];

        let recipients = vec![TransactionRecipient {
            address: receiver.address.to_string(),
            amount: 100,
        }];

        let network_id = 1;

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
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        println!("{}", result.as_str().unwrap());
        let transaction_authorization_bytes = hex::decode(result.as_str().unwrap()).unwrap();
        let _transaction_authorization: TransactionAuthorization<Testnet1Parameters> =
            FromBytes::read_le(&transaction_authorization_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_rpc_create_transaction() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let meta = authentication();

        let (rpc, consensus) = initialize_test_rpc(storage).await;

        consensus.receive_block(&DATA.block_1, false).await.unwrap();

        let method = "createtransaction".to_string();

        let [account, _, _] = &FIXTURE_VK.test_accounts;
        let account_private_key_string = format!("\"{}\",", account.private_key());

        let mut private_keys_str = String::from("[");
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.pop();
        private_keys_str.push(']');

        // Creates a transaction authorization for the test.
        let transaction_authorization = {
            let method = "createtransactionauthorization".to_string();

            let [sender, receiver, _] = &FIXTURE_VK.test_accounts;

            let old_records = vec![hex::encode(to_bytes_le![DATA.records_1[0]].unwrap())];
            let old_account_private_keys = vec![sender.private_key.to_string()];

            let recipients = vec![TransactionRecipient {
                address: receiver.address.to_string(),
                amount: 100,
            }];

            let network_id = 1;

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
            let response = rpc.handle_request_sync(&request, meta.clone()).unwrap();

            let extracted: Value = serde_json::from_str(&response).unwrap();

            let result = extracted["result"].clone();

            // println!("{}", result.as_str().unwrap());
            // let transaction_authorization_bytes = hex::decode(result.as_str().unwrap()).unwrap();
            // let transaction_authorization: TransactionAuthorization<Testnet1Parameters> =
            //     FromBytes::read_le(&transaction_authorization_bytes[..]).unwrap();
            // println!("{:?}", transaction_authorization);
            result.as_str().unwrap().to_string()
        };

        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}, \"{}\"] }}",
            method, private_keys_str, transaction_authorization
        );
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        println!("extracted: {}", response);

        let extracted: Value = serde_json::from_str(&response).unwrap();

        println!("extracted: {}", extracted);

        let result = extracted["result"].clone();

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Testnet1Transaction = FromBytes::read_le(&transaction_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_create_account() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let meta = authentication();
        let (rpc, _consensus) = initialize_test_rpc(storage).await;

        let method = "createaccount".to_string();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta.clone()).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = PrivateKey::<Testnet1Parameters>::from_str(&account.private_key()).unwrap();
        let _address = Address::<Testnet1Parameters>::from_str(&account.address).unwrap();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = PrivateKey::<Testnet1Parameters>::from_str(&account.private_key()).unwrap();
        let _address = Address::<Testnet1Parameters>::from_str(&account.address).unwrap();
    }
}
