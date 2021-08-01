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
            record::Record as DPCRecord,
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
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Testnet1Parameters> = FromBytes::read_le(&record_bytes[..]).unwrap();
        }

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
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

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

        // TODO (raychu86): Generate the transaction authorization with the test data.
        let [account, _, _] = &FIXTURE_VK.test_accounts;
        let account_private_key_string = format!("\"{}\",", account.private_key);

        let mut private_keys_str = String::from("[");
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.pop();
        private_keys_str.push(']');
        let transaction_authorization = "4f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050080d1f008000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b007ef0e8380cfa6aef4f490afeda51b92d80187fe28a5a6fe776f06bd525584b002c0938fa8d138bfc9924a93e6719045b03dacec6a86bf95d5e4c56165267be0339e5458ac6af3be77b9d07a85b64ba8ea930e6d1a2fa08364e1afd5bf27dd5014f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b0068bf9a606ceb997873c394a89339f5f727c251e096f3b84f9cea31a3fca4840b665ce0ed16ed7297d8c0253ee504100e6c508a04f1ab35079bb83bb9f15ecb06de4e2f110e84492b031e4993d80e50afb569fed07dcefdbfd30cd44c2d006702197f4a1c9dfb485e5300b947a1f63d264a8d37811533f9fb0659870480077d06c8a4890859b79fa26191949e0c7f3e8c1b8dfd98e4eb4edbea32ffa9e762130254811bf464869d1b7c163ce276c07cbe35644729379b145828e57ccd449eb5056abb2fffec26415ac3ddb7f26624ebfd6d47597ae1bb6f8044b48a64f68a111220aa5fd4dfd9fd0768d0a23bbf7dc4ebeb9133398b2dab9a9b26d73ff4e079dd052044d07e9612e59e7994ef75abd866cb25715fe2297dcc12901f00d2925ddcd53c5abbc44b0c5cce403244ba35da30cd51abf2516bf63b30f8d79295ea791e590e0064000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00b84d90be722602ba88be985fa4c788da94d66fe76ff42c6de8661dcbc7226f06407005ec5c4af1c178a96d3c3891eb34298e84328732627ed672a2a8e6a0570bc46a00ddd842dd75cdb9f359d9b326c3ec73e86705648ea47333ff5837eb6c015abbc44b0c5cce403244ba35da30cd51abf2516bf63b30f8d79295ea791e590e0100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00580394e5b75d9d34ac8e102a3958fe936ba417b028f72691998641bb5fbd350ae15660b54e4a30a65e2dd917b777f0e44e4aba7c4b88e6e71f96a977f17cf70348101f8ff285ee69d3cd263e8f839ecbf4cb6dc3e1d8581d64a5814337cd49015dfcc875ea2ca4951c105146c5d40a26ddf280d9f347109d77bbc905e07711ae99e8afbaf577af4c5613b8a0c43450fa4f5e4eb7d31170ac642324c1c8bfb213407005ec5c4af1c178a96d3c3891eb34298e84328732627ed672a2a8e6a0570be15660b54e4a30a65e2dd917b777f0e44e4aba7c4b88e6e71f96a977f17cf7037352d6f1bb3d52ff0c7b548f35dd2ea512d9a04389287929e867869f9dc37600c69a0f1ead9e4a881a08e64bdeac22967b1792ba88f13259bb5636ae981aee0208c88d0d2c4d4ddaee0ef6c8dd4ccfae1d21acacf0668922c4437aa333969d4b122949a5ff05031fba325faba7de839fad1384f78dc1e176279884e3a359b4cb034d4766a3ed29ff5788aa6a38b3c774b4cbe73ba284eba158b896ec4276150d0daa85c5d3530ed5c57c6cdb92a3786ebff0408b5775201ebdc89ddd68f3c1b6083d8ae0101bd227ed98da8da14850b90884d5739c1a643fa51c5ded1999784b115b6709797297bb58f9fe48c16b0152a4f11c9c480ee715aa5c199885b75d960d40b66c2900da8296eaddd11c7ed8d38340231c437fcca8134f40fe147dba920b76fbcd922d4ea31f8821f1c7fcc6de9e95260c8f73a67b301dce754fb2f4fe0cce0008b738394df515fa54dbb76ad1d1f49f6bd37e952db121375a710efd52997809031508f9eb3f5a5ef1c9f7a77100596056fcfb9ef63dd8fd951ea1ce25bea1420191ebad09e8e4668130686865bba50218e376ebdd98a5f631778d2f6ddcdb33032e24a08352bb77ddcf1f8da2f2e459afd4d838dbae32bdae44934d60295265106e6ec02e6d81d3ea11aa3578cb3c53b132efe245e357b01b352c3965bcd285057f64f2338e5175063dca85a870ee6f40b8e3e49ed978418a540b25a882952f0d65c458efa12625279b1cadfae88782a411f47ac0c4c7b75e0194ee3fad9f3c12952d720533743e8a3f4d399723b7e614e1fa75e09c45ddc6441a395130d108080e005bcb5f59cd99334b5f7dd77acd51c73c7cedcf4e0344443c55aa5820a350c80a10fbaf4002afc959c2cb288e471cfff90e2b18c847f10b76df347d435af59c0612449c0715b43f5e9e70a421e9cec0e2103fdc3b330c7134b985174f8f63815f4e1ec26e929d95bb257abbc1ef3993c1ab07c7c6f1b37ac68bc2b15d340aaa3f339a14f831cd277f763cc5e6d80590a47a7aeabf3d931c4c43fd7394780ed60903755f65bd072860552a56be48d1c5f7191693b25d37dc2ff1b6decad7c0db0920c56d1bba327686e477deffe801a2278baea1da22ae6915ab602ee04b3c6902a5e06ef408ed4fc4a5b279c90e2a13178dd613c0ff4a5ac9e3ad209306568b0c4fbb4ab01f7666e9f151b06a9b0e70dbd5f5e4dac8a4d26e94a58c2ff198b107023a992a99aead09c57d2f17b84709e86d0b221a59d1d0673ed92bb56fe6050ebe92f48c370fce1f2e21521e9ec79cdcb83895a986901f320d75da961cf40f071a346f71af2e6cbb40fdf72c53599a98588c863fe00a33b67169cf68ec4464025e306bc28e3bdb1aad0ec81fee0ac97e5287bc8f72d41ae9973c21e3cde0580197ab0f8330011bb2c3bacb5ddc62fb1252c0c8d0d2b19c162e261b7ded829803a7927b8e4bf460eec165558dfe7897981003e667c169364caca344de4ceb31041cd1f008000000004f0c21754e9303869804fcdc131745003c0824bc971404ee678645599842318301";

        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}, \"{}\"] }}",
            method, private_keys_str, transaction_authorization
        );
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        println!("extracted: {}", response);

        let extracted: Value = serde_json::from_str(&response).unwrap();

        println!("extracted: {}", extracted);

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Testnet1Parameters> = FromBytes::read_le(&record_bytes[..]).unwrap();
        }

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

        let _private_key = PrivateKey::<Testnet1Parameters>::from_str(&account.private_key).unwrap();
        let _address = Address::<Testnet1Parameters>::from_str(&account.address).unwrap();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request_sync(&request, meta).unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = PrivateKey::<Testnet1Parameters>::from_str(&account.private_key).unwrap();
        let _address = Address::<Testnet1Parameters>::from_str(&account.address).unwrap();
    }
}
