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
    use snarkos_consensus::MerkleTreeLedger;
    use snarkos_network::Node;
    use snarkos_rpc::*;
    use snarkos_storage::VMTransaction;
    use snarkos_testing::{
        network::{test_config, ConsensusSetup, TestSetup},
        sync::*,
    };

    use snarkvm_dpc::{
        testnet1::{
            instantiated::{Components, Testnet1Transaction},
            record::Record as DPCRecord,
            TransactionKernel,
        },
        Address,
        PrivateKey,
        ViewKey,
    };
    use snarkvm_utilities::{
        bytes::{FromBytes, ToBytes},
        to_bytes_le,
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

    async fn initialize_test_rpc(consensus: &Arc<Consensus>) -> MetaIoHandler<Meta> {
        let credentials = RpcCredentials {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        };

        let environment = test_config(node_setup.unwrap_or_default());
        let mut node = Node::new(environment, consensus.storage.clone()).await.unwrap();
        let consensus_setup = ConsensusSetup::default();

        let node_consensus = snarkos_network::Sync::new(
            consensus.clone(),
            consensus_setup.is_miner,
            Duration::from_secs(consensus_setup.block_sync_interval),
            Duration::from_secs(consensus_setup.tx_sync_interval),
        );

        node.set_sync(node_consensus);

        let rpc_impl = RpcImpl::new(consensus.storage.clone(), Some(credentials), node);
        let mut io = jsonrpc_core::MetaIoHandler::default();

        rpc_impl.add_protected(&mut io);

        io
    }

    #[tokio::test]
    async fn test_rpc_authentication() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = invalid_authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String("Authentication Error".to_string());
        assert_eq!(extracted["error"]["message"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitment_count() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        consensus
            .storage
            .store_records(&[DATA.records_1[0].clone()])
            .await
            .unwrap();

        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getrecordcommitmentcount".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(extracted["result"], 1);
    }

    #[tokio::test]
    async fn test_rpc_fetch_record_commitments() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        consensus
            .storage
            .store_records(&[DATA.records_1[0].clone()])
            .await
            .unwrap();

        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getrecordcommitments".to_string();
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::Array(vec![Value::String(hex::encode(
            to_bytes_le![&DATA.records_1[0].commitment].unwrap(),
        ))]);

        assert_eq!(extracted["result"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_get_raw_record() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        consensus
            .storage
            .store_records(&[DATA.records_1[0].clone()])
            .await
            .unwrap();

        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getrawrecord".to_string();
        let params = hex::encode(&DATA.records_1[0].commitment);
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, params
        );
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let expected_result = Value::String(hex::encode(to_bytes_le![DATA.records_1[0]].unwrap()));

        assert_eq!(extracted["result"], expected_result);
    }

    #[tokio::test]
    async fn test_rpc_decode_record() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let record = &DATA.records_1[0];

        let method = "decoderecord";
        let params = hex::encode(to_bytes_le![record].unwrap());
        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [\"{}\"] }}",
            method, params
        );

        let response = rpc.handle_request(&request, meta).await.unwrap();

        let record_info: Value = serde_json::from_str(&response).unwrap();

        let record_info = record_info["result"].clone();

        let owner: Address<Components> = record.owner.clone().into();
        let owner = owner.to_string();
        let is_dummy = record.is_dummy;
        let value = record.value.0;
        let birth_program_id = hex::encode(&record.birth_program_id);
        let death_program_id = hex::encode(&record.death_program_id);
        let serial_number_nonce = hex::encode(&record.serial_number_nonce);
        let commitment = hex::encode(&record.commitment);
        let commitment_randomness = hex::encode(&record.commitment_randomness);

        assert_eq!(owner, record_info["owner"]);
        assert_eq!(is_dummy, record_info["is_dummy"]);
        assert_eq!(value, record_info["value"]);
        assert_eq!(birth_program_id, record_info["birth_program_id"]);
        assert_eq!(death_program_id, record_info["death_program_id"]);
        assert_eq!(serial_number_nonce, record_info["serial_number_nonce"]);
        assert_eq!(commitment, record_info["commitment"]);
        assert_eq!(commitment_randomness, record_info["commitment_randomness"]);
    }

    #[tokio::test]
    async fn test_rpc_decrypt_record() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let system_parameters = &FIXTURE_VK.dpc.system_parameters;
        let [miner_acc, _, _] = FIXTURE_VK.test_accounts.clone();

        let transaction = Testnet1Transaction::deserialize(&TRANSACTION_1).unwrap();
        let ciphertexts = transaction.encrypted_records;

        let records = &DATA.records_1;

        let view_key = ViewKey::from_private_key(
            &system_parameters.account_signature,
            &system_parameters.account_commitment,
            &miner_acc.private_key,
        )
        .unwrap();

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
            let response = rpc.handle_request(&request, meta.clone()).await.unwrap();

            let extracted: Value = serde_json::from_str(&response).unwrap();

            let expected_result = Value::String(hex::encode(to_bytes_le![record].unwrap()).to_string());
            assert_eq!(extracted["result"], expected_result);
        }
    }

    #[tokio::test]
    async fn test_rpc_create_raw_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();

        let rpc = initialize_test_rpc(&consensus).await;

        assert!(consensus.receive_block(BLOCK_1.clone()).await);

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
        let response = rpc.handle_request(&request, meta.clone()).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Components> = FromBytes::read_le(&record_bytes[..]).unwrap();
        }

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Testnet1Transaction = FromBytes::read_le(&transaction_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_rpc_create_transaction_kernel() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();

        let rpc = initialize_test_rpc(&consensus).await;

        assert!(consensus.receive_block(BLOCK_1.clone()).await);

        let method = "createtransactionkernel".to_string();

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
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let result = extracted["result"].clone();

        let transaction_kernel_bytes = hex::decode(result.as_str().unwrap()).unwrap();
        let _transaction_kernel: TransactionKernel<Components> =
            FromBytes::read_le(&transaction_kernel_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_rpc_create_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();

        let rpc = initialize_test_rpc(&consensus).await;

        assert!(consensus.receive_block(BLOCK_1.clone()).await);

        let method = "createtransaction".to_string();

        // TODO (raychu86): Generate the transaction kernel with the test data.
        let [account, _, _] = &FIXTURE_VK.test_accounts;
        let account_private_key_string = format!("\"{}\",", account.private_key);

        let mut private_keys_str = String::from("[");
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.push_str(&account_private_key_string);
        private_keys_str.pop();
        private_keys_str.push(']');
        let transaction_kernel = "4f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050080d1f008000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00909b5182de47a2eddad7a2201dda62ef187a72e9bb2dd743273f8b99d0307f08472eaca728783b35acfac5cdb69a8cb9f6ab7fef91d46860c16570172ad6b70a7f79a7e46d2158ef33613d2e8ab528507c0d40b1abb7f0d0a060e1d9badfe8024f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b002895e8fa85a55c4bb028bc081e229dd2638e5cfbe341a0a413fadc940420650ff643dbe4e3812bbf6c7324914af462fac52834f5d6d6efe08f515d8f3c5f6b014f05f0b441e4d16ae941c683c961b14668244c545fab65638a900e7eb3f6f500fa71c85091f64a914e570b785a3c616c58f601275c0e1a07b8f7d86a1c2241126ab62e62937253aa4446908221cca226959feaf888ecc3bca8151ed72b536908b2f16e37121c4754d0840ad390b677127833a5627587e10d92a4aa6a30fe96055f73bb1813b44e04128039eaf80aa74137314ef8c647b4fb7e0dfed83af3bd0d206205fd88982b655d540d4e61319f14312f8e9a20d4118c4e5a8bc7d78fb93e7920a2977f38149f73a961f27f7a23e6033b737e7f8d2da07a99870f7b28744421c85abbc44b0c5cce403244ba35da30cd51abf2516bf63b30f8d79295ea791e590e0064000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b005ce84d8e9b12731860383e775a663858b402bf1c55c4f7e38ab8c0bb94a38f1009e26c7cfadb671d5744db20db42570e63babf065becb2f66aa67742a78e1b02412fae9cc334ee54958946fb493d37c57c03955bbdef4dd73799ac4c8c9055015abbc44b0c5cce403244ba35da30cd51abf2516bf63b30f8d79295ea791e590e0100000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b006ba3768d061c2d3db16b9238824a4e02cf7856cfff3826418005b0bf9ce7f60ab34b0b16500855bdcc5e74202e8050aed91e07ec00df16cc3d8e9f3a2cd96b05bd2fd058dde62d471c8115c1c31e9401427619cf343bbf640b916244e06f7b034c5d5adc966550d469103198f47cd894bfbfc72840fb9ee44ae3586c9819205e2b184eee9ffee6451a4d3235f3912030505b51d5ea38d1d12dedc91cc92cb03e09e26c7cfadb671d5744db20db42570e63babf065becb2f66aa67742a78e1b02b34b0b16500855bdcc5e74202e8050aed91e07ec00df16cc3d8e9f3a2cd96b05b4d65edcd549143426c9ef3266b085c80fa0cac2e92bb12703e8ef455d8d1604ffc77ee5b0b584d8bd745f20e63ec652a1cb4a3c596c5a4d68ffccfdad30c50108d68be295d26b40d863d8d427c4c7f5c1ac19985097dc040a31141cd390f6310c1480a8c85f044a1acb93a543991fe1b0b25f11df83f5d9ea630758c96ae6b80ec2e59b90a38035911a59ddad14cae28809e297a7665910b30d214b8a7f5b95056f761418e5dab3f8379d9ddbd54e851b12de9d14f58169aca1f21fda3793900ff6ab76ce58b0744525367493876ea53c491daaf3b0290f8f340f98a7fc0265007d9f404da964c54c5ac6984ac78d81fb7588019fca5e7ae8b21155664dbe181239f8809a4c72529082efcafe18d3c134422d3dbb3385ce7886c416ab3b1de001bc4a266908c932eb3ca555fcc0ea7135fbb9b9d8ba929a10cb1376e8636aac0c950008b4a53302ae484aa2aa26ae5f094ded18a7a61a595a6d369e7ed206100a824c0bc1cc732720e8c86d1eaf647ae5c34b8a5af2856c7436f8e7f4b7f5b8d3ab0505eb36780e63a8171bfa7e02ef6a3177ded45a8048e09e1b81cb3a6b3e22cffb0a5fb2a24dbc5775b493e6ddc31fe4b66d6f28cd691f8b71f7eb57691525e33e04e7f8f12a6c212e28b09d0a4194ff9139f69d0a753db111702730715bf724f8085e1ecb267c81d577ef5ea777667976ec5ad115ae216a8ac47089f38ed0cb9907dd4951bde460abc350658214e3691f26da46c303094859aa2252e1f070cb190f9fb92742455b29de12d8e4c5f08bb961a4135ebcca3dc0e8ac68290a87cb50103300278a13aa21014d8551458e2463c7498380bbf384181ea3f0a9830e929366d60024e7c44f7ba6b26377b68121a3dfab0826ca456ef52a56eef7dcd721dd2cae03a1bc4c74248dc39c6216608ee5de5ae52adfa23e99ca8c08ab1e1e881fcdbc5899c62730d68e9e2992b00e75ba2edb70fe09914595ab6399b9d5d55d821d98672a9d08b8341f5dc06ad27b55b7d8681785cf9a4678da4adfb6e35b34384ef50eeaba4f6d036f486fed73cc90f742a1c9e63253a04f62b25f614d2610e83f320217b17b480a2c195c62762939c272c381f8a74a5fdfbed00c002696f3c4a9541221185177dbd0e41e7ee9a4473bd6eb5b18025d353d9ca52b907533434a8d16067a77e3501ac548eba69d69823284175f505d679be95b4da47d4901c405e3111215477c504ba023b81e7cf6ccf9173054794b4020b3099ffb7807e46727b29003b85c118e06157c58067cb48ee253c14905da5ab6e2c272ce6da22e37cd995a08ee1add2bbec4703bc84784cc040feef4bfe1a558522d322ee35b92591693be0257bd6efbc41d9ffb6cee107519b88ffe653753b171ed5f3924f3fef52eebb4016b07a4d9e5fe0f6d820bdf6538e7cfd4d4d2c322eb43f7cefaec4f3c637d3a00b04eead93cf97fddaf6f48024ea379af7389eee9058c74309c42e40fc03b98031cd1f008000000007ae6cb29d8ed10728a8c035f63bab15e70fa7c4761f4cc66b4a9ef654598135501";

        let request = format!(
            "{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\", \"params\": [{}, \"{}\"] }}",
            method, private_keys_str, transaction_kernel
        );
        let response = rpc.handle_request(&request, meta).await.unwrap();

        println!("extracted: {}", response);

        let extracted: Value = serde_json::from_str(&response).unwrap();

        println!("extracted: {}", extracted);

        let result = extracted["result"].clone();

        for record_value in result["encoded_records"].as_array().unwrap() {
            let record_bytes = hex::decode(record_value.as_str().unwrap()).unwrap();
            let _record: DPCRecord<Components> = FromBytes::read_le(&record_bytes[..]).unwrap();
        }

        let transaction_string = result["encoded_transaction"].as_str().unwrap();
        let transaction_bytes = hex::decode(transaction_string).unwrap();
        let _transaction: Testnet1Transaction = FromBytes::read_le(&transaction_bytes[..]).unwrap();
    }

    #[tokio::test]
    async fn test_create_account() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let meta = authentication();
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "createaccount".to_string();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request(&request, meta.clone()).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = PrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = Address::<Components>::from_str(&account.address).unwrap();

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method);
        let response = rpc.handle_request(&request, meta).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        let account: RpcAccount = serde_json::from_value(extracted["result"].clone()).unwrap();

        let _private_key = PrivateKey::<Components>::from_str(&account.private_key).unwrap();
        let _address = Address::<Components>::from_str(&account.address).unwrap();
    }
}
