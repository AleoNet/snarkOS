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

/// Tests for public RPC endpoints
mod rpc_tests {
    use jsonrpc_core::{MetaIoHandler, Params, RemoteProcedure, RpcMethod};
    use snarkos_consensus::{get_block_reward, Consensus};
    use snarkos_network::Node;
    use snarkos_rpc::*;
    use snarkos_testing::{
        network::{test_config, test_node, ConsensusSetup, TestSetup},
        sync::*,
        wait_until,
    };
    use snarkvm_dpc::{testnet1::instantiated::Testnet1Transaction, TransactionScheme};
    use snarkvm_utilities::{
        bytes::{FromBytes, ToBytes},
        serialize::CanonicalSerialize,
        to_bytes_le,
    };

    use jsonrpc_test::Rpc;
    use serde_json::Value;
    use std::{net::SocketAddr, sync::Arc, time::Duration};

    async fn initialize_test_rpc(consensus: &Arc<Consensus>) -> Rpc {
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

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // wait for genesis to commit

        let rpc_impl = RpcImpl::new(node.storage.clone(), None, node);

        let mut io = MetaIoHandler::default();

        rpc_impl.add(&mut io);

        Rpc::new(io.iter().map(|(name, proc)| {
            (name.clone(), match proc {
                RemoteProcedure::Method(rpc_method) => {
                    struct Handler(Arc<dyn RpcMethod<Meta>>);
                    impl RpcMethod<()> for Handler {
                        fn call(&self, params: Params, _: ()) -> jsonrpc_core::BoxFuture<jsonrpc_core::Result<Value>> {
                            self.0.call(params, Meta { auth: None })
                        }
                    }
                    RemoteProcedure::Method(Arc::new(Handler(rpc_method.clone())))
                }
                RemoteProcedure::Notification(_) => unimplemented!(),
                RemoteProcedure::Alias(_) => unimplemented!(),
            })
        }))
    }

    fn verify_transaction_info(transaction_bytes: Vec<u8>, transaction_info: Value) {
        let transaction = Testnet1Transaction::read_le(&transaction_bytes[..]).unwrap();

        let transaction_id = hex::encode(transaction.transaction_id().unwrap());
        let transaction_size = transaction_bytes.len();
        let old_serial_numbers: Vec<Value> = transaction
            .old_serial_numbers()
            .iter()
            .map(|sn| {
                let mut serial_number: Vec<u8> = vec![];
                CanonicalSerialize::serialize(sn, &mut serial_number).unwrap();
                Value::String(hex::encode(serial_number))
            })
            .collect();
        let new_commitments: Vec<Value> = transaction
            .new_commitments()
            .iter()
            .map(|cm| Value::String(hex::encode(to_bytes_le![cm].unwrap())))
            .collect();
        let memo = hex::encode(transaction.memorandum());
        let network_id = transaction.network.id();

        let digest = hex::encode(to_bytes_le![transaction.ledger_digest].unwrap());
        let transaction_proof = hex::encode(to_bytes_le![transaction.transaction_proof].unwrap());
        let program_commitment = hex::encode(to_bytes_le![transaction.program_commitment()].unwrap());
        let local_data_root = hex::encode(to_bytes_le![transaction.local_data_root].unwrap());
        let value_balance = transaction.value_balance;
        let signatures: Vec<Value> = transaction
            .signatures
            .iter()
            .map(|s| Value::String(hex::encode(to_bytes_le![s].unwrap())))
            .collect();

        let encrypted_records: Vec<Value> = transaction
            .encrypted_records
            .iter()
            .map(|s| Value::String(hex::encode(to_bytes_le![s].unwrap())))
            .collect();

        assert_eq!(transaction_id, transaction_info["txid"]);
        assert_eq!(transaction_size, transaction_info["size"]);
        assert_eq!(Value::Array(old_serial_numbers), transaction_info["old_serial_numbers"]);
        assert_eq!(Value::Array(new_commitments), transaction_info["new_commitments"]);
        assert_eq!(memo, transaction_info["memo"]);

        assert_eq!(network_id, transaction_info["network_id"]);
        assert_eq!(digest, transaction_info["digest"]);
        assert_eq!(transaction_proof, transaction_info["transaction_proof"]);
        assert_eq!(program_commitment, transaction_info["program_commitment"]);
        assert_eq!(local_data_root, transaction_info["local_data_root"]);
        assert_eq!(value_balance.0, transaction_info["value_balance"]);
        assert_eq!(Value::Array(signatures), transaction_info["signatures"]);
        assert_eq!(Value::Array(encrypted_records), transaction_info["encrypted_records"]);
    }

    async fn make_request_no_params(rpc: &Rpc, method: String) -> Value {
        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"{}\" }}", method,);

        let response = rpc.io.handle_request(&request).await.unwrap();

        let extracted: Value = serde_json::from_str(&response).unwrap();

        extracted["result"].clone()
    }

    #[tokio::test]
    async fn test_rpc_get_block() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let response = rpc.request("getblock", &[hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())]);

        let block_response: Value = serde_json::from_str(&response).unwrap();

        let genesis_block = genesis();

        assert_eq!(hex::encode(genesis_block.header.get_hash().0), block_response["hash"]);
        assert_eq!(
            genesis_block.header.merkle_root_hash.to_string(),
            block_response["merkle_root"]
        );
        assert_eq!(
            genesis_block.header.previous_block_hash.to_string(),
            block_response["previous_block_hash"]
        );
        assert_eq!(
            genesis_block.header.pedersen_merkle_root_hash.to_string(),
            block_response["pedersen_merkle_root_hash"]
        );
        assert_eq!(genesis_block.header.proof.to_string(), block_response["proof"]);
        assert_eq!(genesis_block.header.time, block_response["time"]);
        assert_eq!(
            genesis_block.header.difficulty_target,
            block_response["difficulty_target"]
        );
        assert_eq!(genesis_block.header.nonce, block_response["nonce"]);
    }

    #[tokio::test]
    async fn test_rpc_get_block_count() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getblockcount".to_string();

        let result = make_request_no_params(&rpc, method).await;

        assert_eq!(result.as_u64().unwrap(), 1u64);
    }

    #[tokio::test]
    async fn test_rpc_get_best_block_hash() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getbestblockhash".to_string();

        let result = make_request_no_params(&rpc, method).await;

        assert_eq!(
            result.as_str().unwrap(),
            hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())
        );
    }

    #[tokio::test]
    async fn test_rpc_get_block_hash() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        assert_eq!(rpc.request("getblockhash", &[0u32]), format![
            r#""{}""#,
            hex::encode(GENESIS_BLOCK_HEADER_HASH.to_vec())
        ]);
    }

    #[tokio::test]
    async fn test_rpc_get_raw_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let genesis_block = genesis();

        let transaction = &genesis_block.transactions.0[0];
        let transaction_id = hex::encode(transaction.transaction_id().unwrap());

        assert_eq!(rpc.request("getrawtransaction", &[transaction_id]), format![
            r#""{}""#,
            hex::encode(to_bytes_le![transaction].unwrap())
        ]);
    }

    #[tokio::test]
    async fn test_rpc_get_transaction_info() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let genesis_block = genesis();
        let transaction = &genesis_block.transactions.0[0];

        let response = rpc.request("gettransactioninfo", &[hex::encode(
            transaction.transaction_id().unwrap(),
        )]);

        let transaction_info: Value = serde_json::from_str(&response).unwrap();

        verify_transaction_info(to_bytes_le![transaction].unwrap(), transaction_info);
    }

    #[tokio::test]
    async fn test_rpc_decode_raw_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let response = rpc.request("decoderawtransaction", &[hex::encode(
            to_bytes_le![&*TRANSACTION_1].unwrap(),
        )]);

        let transaction_info: Value = serde_json::from_str(&response).unwrap();

        verify_transaction_info(to_bytes_le![&*TRANSACTION_1].unwrap(), transaction_info);
    }

    // multithreaded necessary due to use of non-async jsonrpc & internal use of async
    #[tokio::test(flavor = "multi_thread")]
    async fn test_rpc_send_raw_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        assert_eq!(
            rpc.request("sendtransaction", &[hex::encode(
                to_bytes_le![&*TRANSACTION_2].unwrap()
            )]),
            format![r#""{}""#, hex::encode(&TRANSACTION_2.id[..])]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rpc_validate_transaction() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        assert_eq!(
            rpc.request("validaterawtransaction", &[hex::encode(
                to_bytes_le![&*TRANSACTION_2].unwrap()
            )]),
            "true"
        );
    }

    #[tokio::test]
    async fn test_rpc_get_connection_count() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getconnectioncount".to_string();

        let result = make_request_no_params(&rpc, method).await;

        assert_eq!(result.as_u64().unwrap(), 0u64);
    }

    #[tokio::test]
    async fn test_rpc_get_peer_info() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getpeerinfo".to_string();

        let result = make_request_no_params(&rpc, method).await;

        let peer_info: PeerInfo = serde_json::from_value(result).unwrap();

        let expected_peers: Vec<SocketAddr> = vec![];

        assert_eq!(peer_info.peers, expected_peers);
    }

    #[tokio::test]
    async fn test_rpc_get_node_info() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let method = "getnodeinfo".to_string();

        let result = make_request_no_params(&rpc, method).await;

        let peer_info: NodeInfo = serde_json::from_value(result).unwrap();

        assert!(!peer_info.is_miner);
        assert!(!peer_info.is_syncing);
    }

    #[tokio::test]
    async fn test_rpc_get_block_template() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let rpc = initialize_test_rpc(&consensus).await;

        let canon = consensus.storage.canon().await.unwrap();
        let curr_height = canon.block_height;
        let latest_block_hash = canon.hash;

        let method = "getblocktemplate".to_string();

        let result = make_request_no_params(&rpc, method).await;

        let template: BlockTemplate = serde_json::from_value(result).unwrap();

        let expected_transactions: Vec<String> = vec![];

        let new_height = curr_height + 1;
        let block_reward = get_block_reward(new_height as u32);

        assert_eq!(template.previous_block_hash, hex::encode(&latest_block_hash[..]));
        assert_eq!(template.block_height, new_height as u32);
        assert_eq!(template.transactions, expected_transactions);
        assert!(template.coinbase_value >= block_reward.0 as u64);
    }

    #[tokio::test]
    async fn test_rpc_getnetworkgraph() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let setup = TestSetup {
            is_crawler: true,
            peer_sync_interval: 1,
            min_peers: 2,
            ..Default::default()
        };
        let (rpc, rpc_node) = initialize_test_rpc(storage, Some(setup)).await;
        rpc_node.listen().await.unwrap();
        rpc_node.start_services().await;

        let setup = TestSetup {
            consensus_setup: None,
            ..Default::default()
        };
        let some_node1 = test_node(setup.clone()).await;
        let some_node2 = test_node(setup).await;

        rpc_node
            .connect_to_addresses(&[some_node1.local_address().unwrap()])
            .await;
        some_node1
            .connect_to_addresses(&[some_node2.local_address().unwrap()])
            .await;

        wait_until!(3, rpc_node.peer_book.get_connected_peer_count() == 1);
        wait_until!(3, some_node1.peer_book.get_connected_peer_count() == 2);
        wait_until!(3, some_node2.peer_book.get_connected_peer_count() == 1);

        wait_until!(5, !rpc_node.known_network().unwrap().connections().is_empty());

        let request = format!("{{ \"jsonrpc\":\"2.0\", \"id\": 1, \"method\": \"getnetworkgraph\" }}");
        let response = rpc.io.handle_request(&request).await.unwrap();
        let value: Value = serde_json::from_str(&response).unwrap();
        let result: NetworkGraph = serde_json::from_value(value["result"].clone()).unwrap();

        assert_eq!(result.node_count, 2);
        assert_eq!(result.vertices.len(), 2);
    }
}
