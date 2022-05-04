// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{initialize_rpc_server, rpc_trait::RpcFunctions, RpcContext};
use snarkos_environment::{helpers::State, Client, CurrentNetwork, Environment};
use snarkos_network::{ledger::Ledger, Operator, Peers, Prover};
use snarkos_storage::{
    storage::{rocksdb::RocksDB, Storage},
    LedgerState,
};
use snarkvm::{
    dpc::{Address, AleoAmount, Network, Transaction, Transactions, Transition},
    prelude::{Account, Block, BlockHeader},
    utilities::ToBytes,
};

use jsonrpsee::{
    core::{client::ClientT, Error as JsonrpseeError},
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
    types::error::{CallError, METHOD_NOT_FOUND_CODE},
};
use rand::{thread_rng, Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use snarkvm::dpc::Record;

use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

/// Initializes a new instance of the ledger state.
fn new_ledger_state<N: Network, S: Storage, P: AsRef<Path>>(path: Option<P>) -> LedgerState<N> {
    match path {
        Some(path) => LedgerState::<N>::open_writer::<S, _>(path).expect("Failed to initialize ledger"),
        None => LedgerState::<N>::open_writer::<S, _>(temp_dir()).expect("Failed to initialize ledger"),
    }
}

/// Returns a single test block.
fn test_block() -> Block<CurrentNetwork> {
    let mut test_block_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_block_path.push("..");
    test_block_path.push("storage");
    test_block_path.push("benches");
    // note: the `blocks_1` file was generated on a testnet2 storage using `LedgerState::dump_blocks`.
    test_block_path.push("blocks_1");

    let test_blocks = fs::read(test_block_path).unwrap_or_else(|_| panic!("Missing the test blocks file"));
    let mut blocks: Vec<Block<CurrentNetwork>> = bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump");
    blocks.pop().unwrap()
}

async fn new_rpc_context<N: Network, E: Environment, S: Storage, P: AsRef<Path>>(path: P) -> RpcContext<N, E> {
    let username = "root".to_string();
    let password = "pass".to_string();

    // Prepare the node.
    let node_addr: SocketAddr = "127.0.0.1:8888".parse().expect("Failed to parse ip");

    // Initialize the status indicator.
    E::status().update(State::Ready);

    // Derive the storage paths.
    let (ledger_path, prover_path, operator_storage_path) = (path.as_ref().to_path_buf(), temp_dir(), temp_dir());

    // Initialize a new instance for managing peers.
    let peers = Peers::new(node_addr, None).await;

    // Initialize a new instance for managing the ledger.
    let ledger = Ledger::<N, E>::open::<S, _>(&ledger_path, peers.router())
        .await
        .expect("Failed to initialize ledger");

    // Initialize a new instance for managing the prover.
    let prover = Prover::open::<S, _>(
        &prover_path,
        None,
        node_addr,
        Some(node_addr),
        peers.router(),
        ledger.reader(),
        ledger.router(),
    )
    .await
    .expect("Failed to initialize prover");

    // Initialize a new instance for managing the operator.
    let operator = Operator::open::<RocksDB, _>(
        &operator_storage_path,
        None,
        node_addr,
        prover.memory_pool(),
        peers.router(),
        ledger.reader(),
        ledger.router(),
        prover.router(),
    )
    .await
    .expect("Failed to initialize operator");

    RpcContext::new(
        username,
        password,
        None,
        peers,
        ledger.reader(),
        operator,
        prover.router(),
        prover.memory_pool(),
    )
}

/// Initializes a new instance of the rpc.
async fn new_rpc_server<N: Network, E: Environment, S: Storage>(existing_rpc_context: Option<RpcContext<N, E>>) -> SocketAddr {
    let rpc_context = if let Some(ctx) = existing_rpc_context {
        ctx
    } else {
        new_rpc_context::<N, E, S, PathBuf>(temp_dir()).await
    };

    // Initialize the RPC server.
    let (rpc_server_addr, rpc_server_handle) = initialize_rpc_server("127.0.0.1:0".parse().unwrap(), rpc_context).await;

    E::resources().register_task(None, rpc_server_handle);

    rpc_server_addr
}

fn new_rpc_client(rpc_server_addr: SocketAddr) -> HttpClient {
    HttpClientBuilder::default()
        .build(format!("http://{}", rpc_server_addr))
        .expect("Couldn't build a JSON-RPC client")
}

#[tokio::test]
async fn test_handle_rpc() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Perform a new request with an empty body.
    let response: Result<serde_json::Value, _> = rpc_client.request("", None).await;

    // Expect an error response.
    if let Err(JsonrpseeError::Call(CallError::Custom(err))) = response {
        // Verify the error code.
        assert!(err.code() == METHOD_NOT_FOUND_CODE);
    } else {
        panic!("Should have received an error response, got {:?}", response);
    }
}

#[tokio::test]
async fn test_latest_block() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: Block<CurrentNetwork> = rpc_client.request("latestblock", None).await.expect("Invalid response");

    // Check the block.
    assert_eq!(response, *CurrentNetwork::genesis_block());
}

#[tokio::test]
async fn test_latest_block_height() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: u32 = rpc_client.request("latestblockheight", None).await.expect("Invalid response");

    // Check the block height.
    assert_eq!(response, CurrentNetwork::genesis_block().height());
}

#[tokio::test]
async fn test_latest_block_hash() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: <CurrentNetwork as Network>::BlockHash = rpc_client.request("latestblockhash", None).await.expect("Invalid response");

    // Check the block hash.
    assert_eq!(response, CurrentNetwork::genesis_block().hash());
}

#[tokio::test]
async fn test_latest_block_header() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: BlockHeader<CurrentNetwork> = rpc_client.request("latestblockheader", None).await.expect("Invalid response");

    // Check the block header.
    assert_eq!(response, *CurrentNetwork::genesis_block().header());
}

#[tokio::test]
async fn test_latest_block_transactions() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: Transactions<CurrentNetwork> = rpc_client.request("latestblocktransactions", None).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, *CurrentNetwork::genesis_block().transactions());
}

#[tokio::test]
async fn test_latest_ledger_root() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(temp_dir()).await;
    let rpc_server_addr = new_rpc_server::<_, _, RocksDB>(Some(rpc_server_context.clone())).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: <CurrentNetwork as Network>::LedgerRoot = rpc_client.request("latestledgerroot", None).await.expect("Invalid response");

    // Obtain the expected result directly.
    let expected = rpc_server_context.latest_ledger_root().await.unwrap();

    // Check the ledger root.
    assert_eq!(response, expected);
}

#[tokio::test]
async fn test_get_block() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: Block<CurrentNetwork> = rpc_client.request("getblock", params).await.expect("Invalid response");

    // Check the block.
    assert_eq!(response, *CurrentNetwork::genesis_block());
}

#[tokio::test]
async fn test_get_blocks() {
    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize an empty ledger.
    let ledger_state = LedgerState::open_writer::<RocksDB, _>(directory.clone()).expect("Failed to initialize ledger");

    // Read a single test block.
    let test_block = test_block();

    // Load a test block into the ledger.
    ledger_state.add_next_block(&test_block).expect("Failed to add a test block");

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32, 1];
    let response: Vec<Block<CurrentNetwork>> = rpc_client.request("getblocks", params).await.expect("Invalid response");

    // Check the blocks.
    assert_eq!(response, vec![CurrentNetwork::genesis_block().clone(), test_block]);
}

#[tokio::test]
async fn test_get_block_height() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![CurrentNetwork::genesis_block().hash().to_string()];
    let response: u32 = rpc_client.request("getblockheight", params).await.expect("Invalid response");

    // Check the block height.
    assert_eq!(response, CurrentNetwork::genesis_block().height());
}

#[tokio::test]
async fn test_get_block_hash() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: <CurrentNetwork as Network>::BlockHash = rpc_client.request("getblockhash", params).await.expect("Invalid response");

    // Check the block hash.
    assert_eq!(response, CurrentNetwork::genesis_block().hash());
}

#[tokio::test]
async fn test_get_block_hashes() {
    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize an empty ledger.
    let ledger_state = LedgerState::open_writer::<RocksDB, _>(directory.clone()).expect("Failed to initialize ledger");

    // Read a single test block.
    let test_block = test_block();

    // Load a test block into the ledger.
    ledger_state.add_next_block(&test_block).expect("Failed to add a test block");

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32, 1];
    let response: Vec<<CurrentNetwork as Network>::BlockHash> =
        rpc_client.request("getblockhashes", params).await.expect("Invalid response");

    // Check the block hashes.
    assert_eq!(response, vec![CurrentNetwork::genesis_block().hash(), test_block.hash()]);
}

#[tokio::test]
async fn test_get_block_header() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: BlockHeader<CurrentNetwork> = rpc_client.request("getblockheader", params).await.expect("Invalid response");

    // Check the block header.
    assert_eq!(response, *CurrentNetwork::genesis_block().header());
}

#[tokio::test]
async fn test_get_block_template() {
    // Initialize an RPC context.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(temp_dir()).await;

    // Initialize the expected block template values.
    let expected_previous_block_hash = CurrentNetwork::genesis_block().hash().to_string();
    let expected_block_height = 1;
    let expected_ledger_root = rpc_server_context.latest_ledger_root().await.unwrap().to_string();
    let expected_transactions = Vec::<serde_json::Value>::new();
    let expected_block_reward = Block::<CurrentNetwork>::block_reward(1).0;

    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: serde_json::Value = rpc_client.request("getblocktemplate", None).await.expect("Invalid response");

    // Check the block template state.
    assert_eq!(response["previous_block_hash"], expected_previous_block_hash);
    assert_eq!(response["block_height"], expected_block_height);
    assert_eq!(response["ledger_root"].as_str().unwrap(), expected_ledger_root);
    assert_eq!(response["transactions"].as_array().unwrap(), &expected_transactions);
    assert_eq!(response["coinbase_reward"].as_i64().unwrap(), expected_block_reward);
}

#[tokio::test]
async fn test_get_block_transactions() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: Transactions<CurrentNetwork> = rpc_client.request("getblocktransactions", params).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, *CurrentNetwork::genesis_block().transactions());
}

#[tokio::test]
async fn test_get_ciphertext() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Get the commitment from the genesis coinbase transaction.
    let commitment = CurrentNetwork::genesis_block().to_coinbase_transaction().unwrap().transitions()[0]
        .commitments()
        .next()
        .unwrap()
        .to_string();

    // Send the request to the server.
    let params = rpc_params![commitment];
    let response: <CurrentNetwork as Network>::RecordCiphertext =
        rpc_client.request("getciphertext", params).await.expect("Invalid response");

    // Check the ciphertext.
    assert!(
        CurrentNetwork::genesis_block()
            .transactions()
            .first()
            .unwrap()
            .ciphertexts()
            .any(|expected| response == *expected)
    );
}

#[tokio::test]
async fn test_get_ledger_proof() {
    let mut rng = ChaChaRng::seed_from_u64(thread_rng().gen());

    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize a new ledger state at the temporary directory.
    let ledger_state = new_ledger_state::<CurrentNetwork, RocksDB, PathBuf>(Some(directory.clone()));
    assert_eq!(0, ledger_state.latest_block_height());

    // Initialize a new account.
    let account = Account::<CurrentNetwork>::new(&mut rng);
    let address = account.address();

    // Mine the next block.
    let (block_1, _) = ledger_state
        .mine_next_block(address, true, &[], &Default::default(), &mut rng)
        .expect("Failed to mine");
    ledger_state.add_next_block(&block_1).expect("Failed to add next block to ledger");
    assert_eq!(1, ledger_state.latest_block_height());

    // Get the record commitment.
    let decrypted_records = block_1
        .transactions()
        .first()
        .unwrap()
        .to_decrypted_records(&account.view_key().into())
        .collect::<Vec<_>>();
    assert!(!decrypted_records.is_empty());
    let record_commitment = decrypted_records[0].commitment();

    // Get the ledger proof.
    let ledger_proof = ledger_state.get_ledger_inclusion_proof(record_commitment).unwrap();

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![record_commitment];
    let response: String = rpc_client.request("getledgerproof", params).await.expect("Invalid response");

    // Check the ledger proof.
    let expected = hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof"));
    assert_eq!(response, expected);
}

#[tokio::test]
async fn test_get_node_state() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: serde_json::Value = rpc_client.request("getnodestate", None).await.expect("Invalid response");

    // Declare the expected node state.
    let expected = serde_json::json!({
        "address": Option::<Address<CurrentNetwork>>::None,
        "candidate_peers": Vec::<SocketAddr>::new(),
        "connected_peers": Vec::<SocketAddr>::new(),
        "latest_block_hash": CurrentNetwork::genesis_block().hash(),
        "latest_block_height": 0u32,
        "latest_cumulative_weight": 0u128,
        "launched": format!("{} minutes ago", 0),
        "number_of_candidate_peers": 0usize,
        "number_of_connected_peers": 0usize,
        "number_of_connected_sync_nodes": 0usize,
        "software": format!("snarkOS {}", env!("CARGO_PKG_VERSION")),
        "status": Client::<CurrentNetwork>::status().to_string(),
        "type": Client::<CurrentNetwork>::NODE_TYPE,
        "version": Client::<CurrentNetwork>::MESSAGE_VERSION,
    });

    // Check the node state.
    assert_eq!(response, expected);
}

#[tokio::test]
async fn test_get_transaction() {
    /// Additional metadata included with a transaction response
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct GetTransactionResponse {
        pub transaction: Transaction<CurrentNetwork>,
        pub metadata: snarkos_storage::Metadata<CurrentNetwork>,
        pub decrypted_records: Vec<Record<CurrentNetwork>>,
    }

    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize a new ledger state at the temporary directory.
    let ledger_state = new_ledger_state::<CurrentNetwork, RocksDB, PathBuf>(Some(directory.clone()));

    // Prepare the expected values.
    let transaction_id = CurrentNetwork::genesis_block().to_coinbase_transaction().unwrap().transaction_id();
    let expected_transaction_metadata = ledger_state.get_transaction_metadata(&transaction_id).unwrap();
    let expected_transaction = CurrentNetwork::genesis_block().transactions().first().unwrap();
    let expected_decrypted_records: Vec<Record<CurrentNetwork>> = expected_transaction.to_records().collect();

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<CurrentNetwork, Client<CurrentNetwork>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![transaction_id];
    let response: GetTransactionResponse = rpc_client.request("gettransaction", params).await.expect("Invalid response");

    // Check the transaction.
    assert_eq!(response.transaction, *expected_transaction);

    // Check the metadata.
    assert_eq!(response.metadata, expected_transaction_metadata);

    // Check the records.
    assert_eq!(response.decrypted_records, expected_decrypted_records)
}

#[tokio::test]
async fn test_get_transition() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Get a transition ID from the genesis coinbase transaction.
    let transition_id = CurrentNetwork::genesis_block().to_coinbase_transaction().unwrap().transitions()[0]
        .transition_id()
        .to_string();

    // Send the request to the server.
    let params = rpc_params![transition_id];
    let response: Transition<CurrentNetwork> = rpc_client.request("gettransition", params).await.expect("Invalid response");

    // Check the transition.
    assert!(
        CurrentNetwork::genesis_block()
            .transactions()
            .first()
            .unwrap()
            .transitions()
            .iter()
            .any(|expected| response == *expected)
    );
}

#[tokio::test]
async fn test_get_connected_peers() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: Vec<String> = rpc_client.request("getconnectedpeers", None).await.expect("Invalid response");

    // Check the transition.
    assert!(response.is_empty());
}

#[tokio::test]
async fn test_send_transaction() {
    let mut rng = ChaChaRng::seed_from_u64(123456789);

    // Initialize a new account.
    let account = Account::<CurrentNetwork>::new(&mut rng);
    let address = account.address();

    // Initialize a new transaction.
    let (transaction, _) = Transaction::<CurrentNetwork>::new_coinbase(address, AleoAmount(1234), true, &mut rng)
        .expect("Failed to create a coinbase transaction");

    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![hex::encode(transaction.to_bytes_le().unwrap())];
    let response: <CurrentNetwork as Network>::TransactionID =
        rpc_client.request("sendtransaction", params).await.expect("Invalid response");

    // Check the transaction id.
    assert_eq!(response, transaction.transaction_id());
}

#[tokio::test]
async fn test_send_transaction_large() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![
        "8e20bb6d749540012794eecf23287736df7007e4161022733948bd38a8dfade919d4db332f220ba0561e9f7fcfed4d012e57e12a26f401e7f7229f2e95449884845f3cadc65e7cd64ffadd37a87e83061f00f0cb3d8d02af1b35944126ca5481d03975dc7ac9c13a9d534a3fb4b758a92a003339685401f1a22ac7694a1b98f0de2140f54eaca09c7adbe63ba072e53e8e10481c4a877ec9e42236afa6ab77dd84055f9fdc69cbfd2fbea931e13f6167c90d9a604b41868d8985ac9e80d3778e92d299bb914566dfdada4415b0a4d6334b07114a5543a6c7dc0b21d5ef167b0228dc065b50baacbfb1776146487d7f3ae30f7b3d724832a3e815165a1daed56a4019787aedea9fb6eab581b69e9d838cd5112bbf41de54b633e10ead16fcd4fd51b9d815f3150cde6ea3f13f9ad5ce1fc80a0aa3b000c113da2165c89a6bf5c5951eb732d75f1bff0dfadb09cc26c2d52205dd3d0c0271319ca918e5c79538e02d864643e0f3e083e0642c1ce591f0090a10dcfb905d327f23271ff1f2b646aeca3b574ad5b79465a79d922e0b3b17a1d90c76e6ee5f5a2e89c9f8007e26975812afe65640b2cfa87b9ea433c2a085df4c063c2aa77e9695162c20f3eb1a7160d8860344a5276f2827fc493fd867f95edb11db950b2211168ab61e0e58fbcc96236b5dbbadbb5accb7da37755a22ff106f0923a20bd6c0fa2762dcc695bcd633c12b3b85f0a891c0254b5f0f2709f295b0006c129c05c8905c370930c779391c3c12325eaadd3b7e5edd10cfee9046a3aa0664ba81f5cb95d482e6b04d2b6d661789159b3bee4b19657a3a2449d530c64d091b218469eed29dfb4dfe251a64b2cfffdb1bfffe24601f7449a4ea28ba417101418b8e68c484db1ec9d3d49c6762d38e67684046c38f487d01081770fd280f0dfd34d2df8b4415b6d8054eecd2f588053e2f4536c5d959a4924ced9cfbf72c06cf5e2e586146cf4e153d11a224fbb231796a066d7b40e32cbb11ff70ac26e40e49045241ddab5e6b708cfc5f253a94d7b19ec5598444db20bcec593ec2d4e90700000000000000000000191a2e35fe73e3b3ccdadf9a2b3ed8cd90a445ec2b6a3668f5a9ebb162046c94f5764bc8ed042a9885f21e2b583aad8ddbd7e347fecd83186f3e2328be93a8c6cf256c5f50370b17ebb5164ee6b228e29cd465ece3bebc3620977df766abe680d392a162151e5a0aa63596115bcc655abae2612fcc1cc6903fbe36316056075962bb227e6f465ae1368e8c9f882c714470cd0b9358fdc45f574b3c55a30ce7fe446338bd0c3f868a1d2d0fc03ddd5700407211943f9ad10e2624aa07f4ff0f012f56fb7c1ebeeb7aa4e5c2d5796cfbe551cf496f457d81df6f29012aaefa993d2c7a5645ba4559b53cac38e045e5271fd9a3c682274664e9bbf698a65bfc104b54964a79f5740b624b457f95ae4b385331f48faaed05e39c496a430e60d0bb0001c3c4ed155931828ac9859d4fb0d9d1e69ff4eb52921069bd92c3f50ac0ff450dd30a8d59fb00ad79926fd1d0b68a2d5325d7fba44c856cdf9fa005dcfc5f910fd30fb66cb12e69d5f560006bb932aaedfb8eea49ecd5e2a8899267f5aaa2c10dece270db4704bb8cb47387d96c2b3b5826e836a5210ecc864a94f46b2dbe00040250cb0711f0449ab80f7c16761ea77b1cf2d1e31c95689e4921f0a99d082c01a34b7a6c1de7416b1b7f4f14123bab8d7d93dfbde40e047ea307102181df87074841e9a5a1b8043f97ccdee5a5aebb45b2f3e43443420cb29b0461a4fb21ec112be5b58f68bc3418f138a9111cb0fdea065b389815d04936744dc693794675068e5cbaa685d0c94fe5d4e1b960da046c31950645849672dfe8e780226d76e20375487d445d194d0790a29cc870d08e8c3fd0fbba621f6cc4004627b47abfe11020091af2e6843e9f5976878349b1e2124463707bae8fee9b0cf25e747c24eb000fe6bdc08fa100eed46a2f436b0c375b3737d9142da400fd8658777097a3a80f84d5baa5cbdaf30113920fabeb3c2acba1880cb73ac39727f15b2ee14078150d0184adbf40aec1bb19ba9cb502f851d2dc2a067d60306719f8a92f0e6718a70ab2e2a2911dfbc5c2161eb5729ea21260b5d335aef8e5c5fddcdfc78a07f8e30e8f9bfa6cbcc7e6116adbc0baf5be9abadd58fc8f9c0bc3aa5c241bb199b47d02e65e56845b70b2ea152913866aefdea9e2cc917923221dbf62ede04b2748b604c9164bd52d84127fea7fa38eb48998cea1f1f217b6102561f1489e235c0e2c0e0ab0305192708935b41bed6e39bd5e7e0e7e7436ddef8ba62a43f6d71b446201629cd7c455124a79d83ec6e9669b3a445f4e43e90947209ceef0d1fe773c150ae0045d2b91778e5e2d6441adb44e13ba6e99807e6c1fd6d00638be8858718910000000000000000000001fd5eeb4268de83796462ff0beef540b4a7f1ca15b8c92ba671b6aef55fce703c49e5d96e92a23a9c3dd1a801499855932845f7e156cdf15376f037ccc6fe293add6f6b23766d65ca2f9b3b8a23375af77810bdd6a1cfd47ca04a8eec1165d802b3316a5cde358b1025f332927ee779a34505114a433376cc80508f6f623b1cb75dad2050419dabe9c317024789ebf0fe299954ba11d27b414b95181f63c4dc1823c6c71ad970cd736faee822a06d310badbe46022248974c4c3b50d7e9bf8004c79ff358815790fab2d2aab7605741c478b5e249a7baa134feb0f752ec0ee9ccbb6d60248ea02480ab92d8c7a693f401c1c8c30394c0656d82eb0a547b2e69d6c890fb95a73b04379be6cbc81ddaf27fa03f2905c59927564a7d9700992020101787194d3afeb5eebcfbe9d17915c8f086b79d55ce631e4f1914cf2193f63d70c424c918998f8efcc95d54616f99b9a3841ae18d7f8bbc4a1679d7094c6fe4f0a04765ebab393196fbd3ebb1107ad722220a6cb79606bbdeeee52db53dccf2d127fa0fc4373e002a26f1dcfa67712e4eacb7dec9867583efc4373edef6d5ece0b3b99ddb13e80d4326272785cf9207a222e7fa623cd9bacba138b2207aa2df51059fd92be16c633bb708fa9da5ef19595c1e7f9a5da4bc00e760b31454774550e944360081990721ee48f1f018cb7587161c8d10b6e6a7c97f81c3d06875bce0994d86a02b0c9fdad67008bfdb7f61360159cae20ece76412b1e57e7e1c7b381291cb8034c2021e5868e82446d485681e1e8273d622eb4b5414e6ef40949b6812b394fd87a74e11df94ccde360a3037c03f07d2165d9a28bfa51479d5d646620f2598176f2ff00439916828aa017bbf9d6236959ce1cf6489cec4fcf2a29e13039cc91b1a1e63d990c56c7ab68db7dde939856404583b3a039ab6a7023c4a270be04e54e874213ab2e1db4f1a5d805e5ab170b292a59d9300113537a0438b1200942139df4dfad7d6f16e93678ff5514656297333d3a35b6d97140789f6b214032f88506484860ea7c29f1f088e31edcf36b752ae396a2ca1c398331b1b3982094043da2da1aa34988af9adf1a756c44ecd14bad1f2c35c54be63995e3397e71074b131d3e57a3d6284c3773f77cb318b857d66f1946f2127e4af2c1058eeeb0594818db65a193eaf116166db9090738ffc6ac33710f1a02ffeffd2519453c80f020dee31afdabb52a8d9229c5fc2ab5cf4d5b916e9a39029164d712e032fec04cf6634f8a1ec004348fff75760de8bdf93c242f0d60439e683d3129fa8a71007d4ad82f3edd21468815cc8b47a69a97eb82e0c183ea6de2c68c0ec3a5199300a000000000000000000006e065835e747dc9b655d7cf643c0e9428fd928e1813f47af98278363bd889c4fe8f8dabf23dca2fcdd115c9878ef779b37fee7928251c378fc3abb91fec35a2d20b5dcb4d9c4bbbcb81d848dc7ce1bc7fa5431a6476ec43034b4c356d6567800de4fc1becfebe803a586171249896499087802e260af9a95935a9279c7191ef8b5c6b2b5f9026b8bc59152d8020a2228089e223e828fd1f02cbb8d25a739d226171c191fa6ed9569b3cca9584f2c48d97d11b72f88e611fb465a74abe6c03b00b233eda81daf0d0e18232d82ef1d8f641e35370bc71077d31e43689bd601ab6997d7482aea7cbbf6d0ee6f08c98dd99201af2cdf964d8b54675ed135159166581040e7779fd4030d92992f18b976fa5204fbf9438f61ed36625a23cb34c4088001256d3dd5ece3ad3e6d080c613785221797019c6f056c58c0eb0b71d5c6a3211186741f85bb7af72949f65202006fb9ba84997b5edb4e881e61273e61ef7a150d04ef824efb9895fef57cffdad9eee441ce4fde8de2b8bc03ee4e73e09c1169104027da87628b7dd19dfa948332f32fc07bdba0e61040f852e57404a999940712e66f25b29e3ed1f7913e93cff3a9dc77f0f48f716732ae182eef380049739202db1ab9022c48acdff0fe57f379c0c678601afe073520aa9570599e0604a5890ae15135c0381b32a5457ff2ce52163551f5f031cccc69bd8be09d5f9e63c00b0df684ab04816053a09abe3d5231beb367e93d4bf5b0116cc4b10771ed2ddbe30d5a9cca63812276ac58fc330537da337fb27f6bb5e2a244e27aacc2a513a68f063656141381268ea2d10340abc612c6a190a3a050d513e6f140ddca27dda2ae0bb74eff78f0e786e0cdea4193d8d92fb08c1ec6c5ae04b47dadfe4d3f6319fa0251779c56c0129ed94d07acfac50863c29343a65613da0c1c2268d574247c180c16f5482f1c286d4823bf77fba5aaa7b2a4656dda89c014708ce100d04d28f905d34f36b381152b0dbed1e004e356ae80553ab17c108e7c7657460f60894f350e4fae171a5fe0e36ae8c6b338ba11c16610515f3aaad14fd63c1908f08984ab01a9978924e64b82001e2b5ba198522ae4d51100cc4325aff197909440a77ed406b42309271951a2013a4c04d0800d3b2fba20f07aef4e97a161cf05e23c5649005d92f37d7e94a03d850b56415ee29a42cb3bb4c97fb46ea31615f68aaf01fa0d6cf53c50d03b97cf89bba9f007af49f9ee41d33c140a6e9d8f9ee49b9a2d4e06d6841115b22b17574aaf47652f9208435b76f247b98eab547ae8efb7f2cd11053d6de49f7c94194200931835da94f6975fe412514263da56fb525cf0009fe10600000000000000000000c56fa9ee5bbbd9f2fc31b15ab10a154bb8ab01bbc0287f0a6a9c009935000a2ca275550b5f8065f918db5442d5c58d740f84c18bd9a7e99f8ec71aab21a39296c37787de03202a6183bc86d3d3602b1ce6e07ded34c66919dbb75347c780c6804462dddaf97aee46481e64e2ec7707e8576801527ef0d91e91a4bb8d81aca2dea3af4e4a9e54932244e7f7af491d01c4989c2dcd7957e33952db0b0a7309516fca62c7dd535a69d0ba9caee2d9098f876ef6a929aa3578e8fcde6f83b9d3e580f75b6b7e74ca8e9f390c3faa92ddb4e9a0906bf43ad71af8617060f03217002f2af6cc627ecae1e59a695a70d3bc7f23b763dd27bcc5fc1fd3c3b23cc4abd05610acf279a780468c3b8cff71b70dd3cf10861103c56103aeb4f512b106cc9e00011a96aef999dbee0857de7c9085965c4557eaf8840c85bc2c60e7e02fcaad2b09ac1b4302b40f1df43107e4526da002c1f82810d8fbbec072085d6ca93a45040a9663bcf7730963fc9d5f52ac62514424ce5bf3a474577a211935a64665851502e5e1c27b9d709be746fc65234cf7ad269f3e8b8924127159b0f5e1a2cf09091172d6404ce2279615c78214f774353b06ecfb5ea639cb70f32847670f689f540ab80fd28fdd45fb36f2e8a1dd2b046e95a29164a2c6d98d543f1fc18481d96e09f12112b5f8aa88e145221272993fc6965d0bca435f784579d97383ce1bf959040a93aaf282f2d27814be1042f64248e074caabe2e596813d1c4f9d422b90850618f5e14ddb61ae8d0ae34fa6cce7722da41a8921ce2ea300eb8f3e84a33d350839d9cdaed6331fb8d3fdc27a466a9a3a6bf3c64d04e7be39b2d3c361e2d128058c169e279736c8f0123857fac07134fee0e28b39f3bebb1ccf25ce2484f76d044188615e430884c11ca76d4c902085be0e635f8c6b75eb37e7e9f6329ae5ae0bc2764219be937647495eb03873aca832b866d8fe813dba6d1e114732b9d3a90e3d69822c87b531710db599ca726f1fd5afe55a347329430c68cafd6487c21b0c17e69746ec40b378428aba5b5a3a8b6dc86d261bedc048bba05cfe786320a70a012d90b2013d7306b3ec04518cd0dfac2b7879f6a52af040037e6d1c7b8b060bfd0eb2b73f5d886e0b80047dc853dde34196635a579b85374763020c6f5b8f03b8bd5577d24924e0863f2625f70b4529fb2776e079618ff5072606a4f653ed0c5ac64b533e3da04f6ca410e390b3fa3f090f2745dae6dfe38e79c4c9b716570b07f4821a37597dff80ea345652a77931a86a7109369eeabd4ba3a00ec3a67e0458152d7e989e27b96d9aa2508643f5b78fa8722b49096c8b7366b8c8123b410200000000000000000000c9a69388e0f0260ef9e7e50555df210de702562720fa4af14ec4ea45c12de59720dc9aba85f7b647cf811478cd78ece993ceaf288c8ff2e569608e550606966d6986498cb98d381c3d1d07dfef79a63ec78f8e57e92354f7871767fe1692ac80ac841fdc90eec12027ae6752a0ab055709efdf32e514e8f3914ec279a4a99dc900318097e6195b99ef780fcc5dd6225528039b211f3c03e500f89b94e0dec3b2ddb38dff2ef123cca301742b33c3e48fb7023ddcb166b804f007a26823f321812b3a896a83293f703b5604ab3c2203ff9049e248cc9aae05b5a7712e7307b841e8c81467ecbdd8f75f4d706af606a7bfa2d49c7d6e98e8ecdd940d2d69b40bce8c6814f82c17c35dff4b4a1a772fd47d244668ae5b36f077eb9497a984f4830001e9aa3935a1f4f3827effaf6eb9145661e5c7c7c29f0b12029179b87d3fc8280b314224f0ef62c81e0cddecef52f5fd66f4dc6311dddc16782033f4362dbfae0704de4034105893c0f7718e45c658fadf7c3f2037441a563ac145fed89f577b0a7986215b706d2d93bbc135fc508094e2ae5dcc7b5111a9be57a2011b8df1d002c048808b5ac16169f3729f1a45e4d904bfacb2d563e00f6575f44bbea1db3906efb7a06bd2a3c1dd3fec07ac31d4733d952108acc497308b4e275bf5ee78cd11dfc6637d9fa1081c4ae40f5ed7f4513e0ee26d3413d6abe60ac14191775fea03d07fe42086f83a811c618f1a8142aa4c68be1904bca7e81e7264db53c924440504d73295eb361273d2c37c4c747fb4f6eadbd865fbdea8cf62aff29a91aaf60164ac708279d06134c14dc0ffab2e02ed466dcff8da52120e11e29c3a2b67db0084202340bb065ccce240ddc471f593667248b64feb09eefc08564d33312f360aa7a3dfd29d7bd9a35be5addbb01bd5f1797861deb52d76fd0615fa3054240c0506cb7e8ce5d831bee06a67a2be4853dc3c0e28a1e33e0df6bb9c8cc96d97bc035156f8a5e0e01cbcf39586157355acf48cd48c384777379928b8f5c5b4831c114ec14f17dd07ef4f98ac56d6ead21932bf9bd6358fce98f96fc0bac98390d90527e0765ade1187ef55741ad61a8c8f5b82461b4e83491b59342edfea92c75204b24c8ae7398d3327f8746d20f64667444dc2747a0b8ab8390297bce65c7fe70667ca0b8a59176be7e6f534125064b222c55037eea9deefcf3d837b0fef7118077e1169ec8ba2447ba8bcdbe1059968c5dfd42877b4b878917ba115f61ea6630718f3871c29ed737c637605ca8f14a3b4a374f4f5d41f0236d5ef79fcc71b1509fda0a2cdaed9161bef7ccd94193869b6b4fb263b1b3d718f76f4d94a47544508000000000000000000002c6442f877618a4d04965f5fec0856afe9d8db321085e084c2988e7f00a8933d9ba71ecf3e7349b6f49eb3378b332029ff78fc88db6f7dce8e318188f56f2e50ecdcceb79aebeb8f0416347baaf0b82e15f69af0e0dc9a2bc2764c5bedb1e9003d02bda5ca2fcf8c2d1595f626e196685923382b46fe7550a1bc377a678c640bbbbbbc11713afa7a5c2f52bd5e3592a6d68f1ab91de959719a07a56a028d83392fd479196938c7e312c6e92f0e8be8c410da32ad582defc473c699277f689900ca120e2554866868b40fdeffa4f2e6784b1b2555ed7ef614392a41fb4e87fcfcbb7ab8dded2c56b6f096790258b760754bfbc5e564c3ed4930faf55547f52f2d8c58ace5ce05ad04893bf5ec8da6c2d60db6e04d66dbf417b6334c43d22eea0001c0c46ab29b483046c247b333428c5dd693c54ad0006c918201afd147485b5904eae463806226165ab92c0d7e1c60c8b2d634539795837ff57d03f2ba608d5f0e32eb432c420c543d03e983047570b6254e57df548884e71ce901d717e2fa8901b68c421a16fbb1b89a8f7fb26c1e9341bce9392e9c40e9d7827eb3f92a32970dc0199d065a448cdc020549d0ad49528776b2ebb060a35313f9bb174fa2d03e002447931e31374a51d4d72f8322db922237ebb1688726b5e3c207242d6ea076076681c08227078241b234219b4099073b428679e7ba85394aa56a70a2d8148c08ca15395788c75644742c0ed94351e357da82ff5a6b54d53c7bc1c59d26a7e10513a4dee16861443071bf4639d85da0abaab0ee8334c7214064405cef37dd0c0b2c33d9cffcaff58d9188bd47f466b16e8c6727196a88686f74cebdab83b62b1241bc353cea8e3cf1cab8a37ffcbbc7dd193f30194ce23fef770f41ad4f4b3c0319514a7711d2e9b2062a653b3cc601166977bd20883859e624b76764a4e52d068fd9ef70d5cc25f78cec4b967cc839156a0a9bdc87bf89db6c3be02e49e1fb038cd19c5915c7d9484005f035d933c2fd203ed59a8c5209ff76f9350b4fd7a50e7cb198b68ec66ba34539eeb41546625fc0522faaa8e55678b80c881bc4eb930ea34e3646bc905f7496791a201e13ff056bd800df1815a79cfdd8e34247cf5e073539be4d5ad709aacabafa47ccedb0009093eaf6f1a653968b8f6b951e6ea90f28b22ddc852bc512c32516cb787a4e67796fcdf4a62104870095066049a3120522c74e3300fc8085c47f3f5927c2d8db2e3613164de6a7dd00e851c18a92d2020e6819dd3e80dd85d43dabbbe7f6a7dc9cfce8f33b81d972f0f85f3057408b015711a48c8bd803d2d2aa788a7f504011c0963a30d419d054972f1be698c43700000000000000000000005617a4a38821242b484e4a4e1669faea3f1661f3d1742c1f38383cd02fb5d9330fc19c862bdd5ff2df502c5de914ee2fd490d1f8c0fc9b02924b5a6533532393d7ec99f92bcdd0179343a37bf14f995691870eef0e1518de400692eb11332d009ce88ce144614e091fb794023a14de7e062fe56ccbf98087858ab8dada7b302931956985e17bdefd87e00a0c3babf5e9e4c315324c2c51daabeeb2627a9b2ce9ba40335118caa39526092b2d91acefddddaaa879328ecfc54e6dfdc1042a05806e11ccdc04b789d6f81b364eac6743c31c3f7bb33c9d681102c85fab46b56ba41e38e3286c7f6dcdea43ab81b80437ffa06e1ef68bfde9c27766b005b9c6a1862447f2206a0a973a40affa11259c680eb1d950f27ddb1ba8a0c142d4a643640001f249d8e63ff45247d4a304398ba3a55d3e5d2c943b8a6cfa9f9010e24391c601d9d997f821cb5d3b2275564847e7016ad030a30192cf01ce7f7ccce95c3460007d7aa528db330589f9e3ca09e6064b43c781d8bf1f0d0981e270d4acf733a30669a1dafded61de48ad53cf39df34416a9803b75bbf39d7a253362d2812feb10670da00c0e5618a292644f019dbc742647cf6c95fc837efc1d1dafed0b94c620b2d39d0528b81a7b1b49ba9c7684d76a6f9371dcec08fafbb39d909bda561d005897c08697e2917ae771aa0e3bee76d46accbea9f6bd0f848f0f7dd4be5b1010228bf60f5eb12711a1c2dc7980acef07ebcaf2fde168038c9ad7cdab414352005518009c5075429a858c43e6e8f2f39696585bffe263fbba5a5b34ee0e1f2dc02abcdfae5bb8de876aa880ba9af5e3ed75390f6cdcd0fc0963fdde394bb7bcb0a3f6e458b4c95bdd65ca20355b4ad60bbdf366d0afa1f135a8827768de0d45f0bf4cb52c2c1a3800a7e46ea21a5a6a4ee6c08296e74161caf854aa8a0a5f0db0ac3f76f882af3cbc54a6836191f6aa36e0cbfb94cf00ac24ee0cf236388ed150b6bb492ac0201e4334fd440a69f56ebfe42157bb900fd74ac6d5e6807dc4a9d045099235a0f642d19dff1308a8f0bfd969a19476f1866bf6a5814f2806062f6050a77652f7fdfe1382813b8159e21f785bc9dfe59692a498d128f97ffbcad1506052a5fb6257c361ba01b5eaa86e20ed796991c9b07320ef7f69f18a4ce0b491212b370a804d57adbc87228cd5f4b6adcedf76dc031e2229464ab6a7f2cda5e0e6d71ca7fb391202081fa849c2e2f284bca3d117e56f3e5ecde38dbf9ed8145003aca6cd5e9da44932a61b08cfe944e69f7bb0653d3d9eff8f05f63163dd1e5097570116480e70bab58f869db10d95d8bec04a9d24e78a6bbf3d15164261c790500000000000000000000629d7554c11f6bac004711662bf872d3f53d850a4c88c4a38790253e03cbee17b6ae1eab11abedf02a0b41443fc3a0fed165e7dc5996eb19b593784efa1d5ca66d885464e2c4225aa681a1e04866f89359bac81937d33dd93d73dc9f63e4b100a584de6ed9572d96c2754ceeb6c46594c8633fac739e0f89f91bb238cb6472aa6be1153ec7d0fdaa6661b85cf3c061e68dcce4245fd5da3978e94ff5aa7a5141093e2bca05a5c793cd43bc819ae6ad691ff8f8146bbbd02a20af878827054e8088352c27c24e62debd98ef35749951837ea7cac8d1615c77010bf6cf921e494b79dacac0de4c0acbf37ef0602f2de71710ccf9845f438d8dbd9e00d88317fb15fb6998ac9347dae5818c4d5f0361dd6489b5c8be848635167cf7536301d9b08001d42d3c992d67d7f2cb944796ca0b509095d836893dd56bd0266c8e0698ea03049c02149aef4a378430a361679083d7be9eee8ef75e1e843469185f0794ea760754f05a269d001018e17b84cdd68c424074227b0bf62cb826ca8264ac63b9f20009f874663fff68fb446f5c2b9ba4611f620349f757e111aa35ba47c9131eb901785a5449bc2e6152a974fa3efa039b8e6cb2b856116264a09789ac6ec278850964b81e0a3eb7279845669806081638f8e7a7e6e55bb5216373ae46076d761f0548d03d85a51ea032b36a5e7e98ee57322bb9845dd3a42e6d40f3044d81750b086f85e739fe0bac58e0e95d68db45a19de67c867957611c1a6ee7a47cce8b4701e794173e2234be7113af19d86dba4684d05b18b8ecdf457367ef1aa41384030b3bbcdde148bd17ab46cb11954da0bc48b0ada3b7ad48d14a28d9f97cbb2c2c00d198b5731e56af68decbd0da36419f18e965b10ed329da40a5e3fb79210d3c12a7617e44438fa501d7ba2bc0d496a32a8ca813f71a0d157346df3f0ffe404202f215fbb9a98e3ca60d8417ccec89058799911ab16c881775cc02ef0738bc29057cd00078ad221b87dc504b079740e3099964b57e45211ac52160836094ae4607bfa8eb10176c106f37d64d71e6c34cd727a58619cea4c17b8ed89556f0e16111e734ddc20326e7620b8c58bd882eb809966c72cbf99ed652b4ace840cce4b102106f2a04ad284523eab95fa05ecef0ff558e5378656035542d1bae6495838a11f6e325c166f2abba9dd043be4bf42e35a3ed704880528031353677fee189d010c456f94797f4f3480ee31b2b72bff098cbc32c25fc7d6991508f56bde7454e044a092686f0029b3a24c575607a8ccfb762235316fb05db0d1665e093daf7eb026a19b2094dd153f245215d98452eaf5d03ecda611b9a2f1c2b529c16c6e0560a00000000000000000000f91a98f545742418ef3ad741fcdd513b425488f2fa5187337750accebeae591a3c06c10fb36d486b2c68df4f1102415be3d3a1c3eaf2bccac23d7141198980cfe208df0dfca8df935ac3efa04bc73e996bae5c5a0397d7a73a84d19043c883807a343a176d5c3bf75c777338facee0f63a6694d80100e3561dac77445260e4b1c6bd271301f9a309d13a3dd961f518865e9d76852ad432c2698696a2ef6cddc9c3338b0633f582f23c42676c2fa80ae1807f64796d7851ab6ce24f18ae6568004551b98fea1bd1fc7fbbf197c493203f9f9c5b8f1a1f963efa71365d7a6018cd7220d9bebee0979ed1b06d3717ec7376fddcd72305c66e2622410c134eb831ac4e9aff016b49fe5715333cc6ae1d839bdc63a55ace9ab4ca4b09aecb5065e98001af8c38ba548ab40b27efed05503f1b722fa0d3c498eeb68e55fd34a6d8ce0602b766d42671cfc8819cb97ac0efba983551fcc84ac08f0e1c3219dc06600c5408e7728de9a3d2f939109f8a825607c718773d082d6d2e85baeaa573cef01dec032c26f24e36fbfff03f87370899fdb9ce0725150e68b80039c0e2cbb67038310b30b22d44696856d591b59b1520cc93a7db612e10730d69453c754822106d0b12151dedb1d0f986b66c44988444af0d32ed05fb60fd45f65326702096fcda21107d89d0dec32f7fd8f7717b3ee4de7fbe4fafe363437f7f9115f563461408f107a00788bf4e8d74b46b291a95bb9c511e825f5a2331777398dfd771cf82bb3506b8202524074abf3df184609f1fa86bb54a10f4b1c9a2ba5a1713ab09ac61fb07bfbe035ad90a12d026e294644e5a28de95f569606a761521761fb4103155ed0f63e05863e853fda92331eeeca6bdac2dcb7d12bafd61577e79b80b31f9de0d0e4390479ad3a4de37ad5705afd8cad1bdf48466544c6223796c0426232dfc01058762390a26c1d3f63492f0d81d851f38b31a30dada4664a76bf852d538fc8b0201f13c31aa58063a4f3ef64deddfe47bac940a98ea748f460dfab1b82377d00e43f0216418635cdd075730f4e37b5900224ce619a7101e2ee25ca50354a3fb0a28238173a3c277ae6a5fd8528a3cad8815abdbc265bb7e5e72fa6bc4f05cb806fdf3fa0c99605e0288f1ae5d6a1f2dfc2f3de42bfa07718be9f9a071be769c11010493788a2b2e34bd92b6ac1a3571da8dab1a097bec3d6b1ac54d8bd0980d0c071e75adba8fe3f2d82d172aa3fd06f8e0d30d3398903f15a44d6d3718011a030d5aa00a36ee8b5a5a481996aede662b04e4f19f79259ae23cab0f3dac09a80a757ba3a438c1f8713bb2e3fa99a86c419db76c55e7d657bbdff7c6f87625580700000000000000000000628d5f84d0fe2dd60a60ee8485158990c3432e3c91d3effd73aad2fbea7e7114ed7579683579d6bd8a618dabfc169d8b96b841e0a236e330d801049fc02908b6006a0c67d1da69e87d517957a6c7e91f8e2cf62fa75bd4282b1622bb905c3800f57f7b0638e9dcda1218a2a57677e308e311ec7b448b468660a7623a810a481aff729eebc3ff6404ce7c2c478734b14210524b0d65125c0bf9034f03b558a9d4f495797cd26416e0edff146e7a7b005db7cbb6eda70148cef4c9374fe517940058f60e2b3f59de496d035a4a0247f7c8ecb439ac3a61cd1b31b9936a59f3c32e70d4da87f0a12ec1f52798e9e0f1ba4456c722e21b0ad738ce5d35ee5898241301ba5fd5744efa5121c9f73116bfe58bd2ef35a6bf6632b732fb2424ecd6e38001d6bf5fc4c95d6848eb143ac8ee1a662a9bfc7dd332fc8ce61777b512b1fcf90f7cdc8627c716d2c87dc767c0b2b24b1183ad9a9c9a07baed76fe1464fc0e97072a6fc53651b11ec7c9f09272ca06b2fcc7bbd8041d6f475de66dd60de2af0c048e7a1cb8c217b618d8a485b0d0e1d330a7e198ac91cc64dffca5acd63f9d760b307e740fcbfeb009ce279e7a056c583ca98107d7cda626eeecb0e2a191870e06518a924c23f357905e33e4ba74c76d169a55722815297bb01048beb6240c22081d54614bcea202fd82dca735235dd709d8fcce11267b3fb6ae57870e8e7675053b84bba3d1d44c97cdc0bf4f0b1bb83179fe00df9ea892b25a9b5f609b15e40ecb028fe5f9279022c5cc42957cf800da836db8ce87f8006ae7762f7466b558035eaa1c49dedb91859654446902d6bd7b90855989f7d66ce103bdf51b63e91500c8ec8213b07c87d1dbc7a8ad1d8815d6dcd7cc81c0ebb74a68741677b26f82041ef36c0f91cd73e7f70fd25f202393cbfdc7a5659262c62105e08ceb1308970c6b69795605797d4424ab56b2ca0569ec9846be6fc89b6b31ec898d79fd901f06d512156af69f46bb5f20411b0ca9694e23df8908ef644c9d555f4987549a93069447ff5098e28baa038c3f9f83829bec4c748b9caf446a97051ae3a7615580031cadcd9a463200d935cf4d6bbd95815108e0bb910e12cf9d491c207d127a9f024dbc0148726545a9b0b726bf0ec6c7f820e4fd224754801002c2558e0c5c1d003972651b9471c7d900189dc424c32db56872c7d105ee8fa0652ec4b6c8caf00a9a894753d965483ff9f54121da1ced7173e8c3eba3c4f6b0950a23cce5cc2b01765289918d908d987ccdb387644e6c54c93866bc20b9dbecae5895dc8b7f7f074f193ba741b2b87f67fc9c91fee809a2888f6ae71fe24edf80462b5d7df6d10a0000000000000000000030d14e1910119532e60de8bc1612ff0a9a0fd7d5c255ce3d9ca483a94ca5a2b28c1ed34b9dd7be020426b4a7cd61c97b01a48f9486988bae516d501c38b9e1fb32b9b804a248ea93880374b747d130469524bac90aa1e13d4a0e23dabfbd11810d9819ffa4116a77fc084df52a829f94c9b9021ce9fa72c5b8d059396bafafc191abd8b3ee58d4cecd4bad3eea0df994d9ff86a327b1e45f68818050a73f00e2668fb9d92dae48d6a9bff07f2a749050863b21bd70aeb4235a462ea0453f8280ba6614bf20c6d71b9858d2af41c2102ec7497c02c21170c03072cac331f58f996da79e2b8286c52c6fae29d02bc967d885ff001b8cd1f854d7c1b6ae704d3569358deb1c902b877062f0d00a72582b57ae4f0badde868a2475dfc3842f4aab8001bc7ce5607e8bf5c07a53ad706ca591f9974e4887da224dcbfdabcba3cd5cb20411aeaba8bdc5970d9995ec86de253e8600fe755a9bf995a40ca3df27d7cb920509385ccafa38d22005c772e6b2a17776faadf68e31a73973752260ce7c1d1201318afcb112da0dbdef5bb37385d406200a03876afa34f14841e3b32179b6fc01fce171759f91bb3b1c15b4e0f965065aebe9db89fc8e7dc60ede5229589c070c6ade5a614565e6f5cbed1514e9032f4d1c8f47a37f6a75114c8fdd6f93c5a50365d21cf1f4580053b313ef5de3f7c31439b51d4443fe70c8d97986009d91f80a4c22ce47fd9553f77c03c222c4a505f4224d980a1d4869d5fb57900a280e1908cd1aebebab480de7ae9e1a6ac40f61c4d95f2e23c1289667bbeda2026e42a6098050f3b43842177fbd8e578cb4c360189b30be33c9af357d5b23636824cc19009a3b46b0a6aec31fe33e0e9ece89a0b46c2ce5745da35af2524c756fdc13150ea0ab9f8782c028e822a049d053b50a275ff9c74672abcf5dad1f52a2a7c5e7088a6027512b7f16e59e3906b1813d5d4cea40b8dcd3859ff8eaa9dbc48deff311c185b04c0cda2e5f770342cd69a47966191e85fccbe0466477cd756f478682043d98760dd429d54a9ec8ed858a31e03fe19de2bf24450d91a0122014fbbe0e1128d472148cbe52e23a0d0373f4b478577e4ad66b0404e57f69c671adff9849081abb82c1756c6c9a5cbcdb53c5a70daadc20ed363adfea45a93111749bd26d0b0c8043f6cdb156ba28d606d5498422112c8ce2a230d25bb17473309f53777d0914aa606f4bb26c6786e3ed64170b9c70d705c2cdbcde77610b24b5f1796dbb0c4c6a715a2f1929297fd28b5304f84d60f8e56770244d084255508db7c492360013307e77bd410bd7982a576187dba100463594f0bd00074a28b42295023cb2020000000000000000000025db199abd6982b25c96bb9fdb9997a9b8643db18dce461f47ef1c01fe4490704f1150bf57447fded95e0afd73626266f018682b68aa5b313aba9f76cc08f2c73a3a397e176acb6adaf2a79d1bb0182da6fb6fc473ce72438ec60d9d334406813984d2d0d9353bf11b8d3d7eb6f4f63c9793cadc7d1f7a841935c433111f847e5eb8cb4a6f85ed58a3b3e202baf573cebe2b00ef17442651abfc9a5ce49a14b62ffbc517a93c8941a53c008f40da26447c942c5d10592d397b81e7c2004ed0009b66856424c7ef82f16e10b9748959888e4133c5d89680c92a4d98f0e0de919bcfbf61e301d613dfff521f8c8807bce3997c9a0a94e74b75cc8ac7ab83f2901c3fc592d07f100461726ba40f521c6cec868c59fff3bfa3b3ae12fe7c59540a8101b115391eaaad5ae9529f367b58260e4fdffb159d6f5361229f35cb16e9bbae0e52146b2d60d503a38d24f06bea094810f60a3426b1ae63429c9ce2136efaad11add85251e3585e538f6b858ded5e2a69fb6388bd380944513d388302c67c6f10b478204561b62ababab7b50f9c78487fabde0da63688680d676d35a6729fba04ffaf9d97da7ebbba19ed3612898f5be4ef64b8d2500f66c51556c900112ab80b1b5275b98982a91849b7a4fcc30573a2a77f96e84213cc9eb42556dff54301053a7e72f7a6659770e495f4cb605a38bb4cd2e069a5ed30ae10db6c6756f58608a08c04a222a5c67a8d2d5f489bc1e4bd0c44a964043d3980e08ec7e99c41fb10bd678df07f980edf650b5c43707021d25f53cfc5b6cd62a3942f0c0d13a2a00a06fb3f7baf21ac13f9225506cf97ca588ef3e225934876e8b8294b125486a30550f5f663348b4edcf65abca540c8602be26a3990a40ef0c904753f7135fac3103ba90abe32d13100765520d738aa4fda64a8e88ef2c1d9885cbbfbecee0e5e0c4f3fdc429b8e3ec4b6c0d3065a51d421d69cd702d8efa5ba5f3cac386ab8000b146ee1109bafe29466ac536249d3b16bcaf9d6849200c17a16015c35f1c906056f4b339525cf3dd12b4213f730892de32975c28c8c2998eb23f1cd05a4b0c20f6be297931b827a65b9c4db7d1303775be250b0d87b696383f562b07d108bf60b79202f59b75f2f5139a169e8d128d994c21db812866cd2c0b2ea2ce9cebfda07d5c02fd041ddf81449547a64423405c492cbb275da7ed2cfcc4484ce59fcda017569ec68647b3017f6cd96199db93f654da2ad77309bec0cdc690ac681154711f694934c6069f606a067b70c3913bf80c79f57088efa3baad256fe7ed831b90caca4034269d5d9d358bc2c8607646acf3d240751d9c55d063a150c0af7bae308000000000000000000008c80ab70683cc7f3973aa09709580a75eb5e9343efaf9415fd8957d3b85d7707d6645c08c92d5c2c530d267ed5560a380c2fbace75a7b807841c22c593e0164c9aaa448488d38b821db2cb0f15befeb9c58fbb04f481e3e1a75cf9c4c4ea12804b8b831236ece375c26d3d5d52dce1c2843607837ca90ed5b28ec11fc875bf69a53645b156aeeaa0f745ec43c48e9984759f346c41f942e8430855e59e4a5ee776b67cebdc99687da259f85b4bb2418390650a6170228ed8679fcb8e0053e0004ac6060f0c4e11809359f563cbd65de7cc67f3b79aaa08fb183e41c79284f598142cda2aab1f800883043d7fd8fef2ae06178dd6a0d324a38e166f0eace241fe0dafcf5af9e09028ed581744b40b0e003add941ec08adacf6fddd0805cf869000119a1843e750dcf10a25911c99119743887c62a5948b60732f90487d607603907c7d7640d1a593743cb41a847f0aa35012e487129a74b7d860bd655bc24bfee11fe574ff35d2b237c855b1d878bffe466d05ee2315938731c81f78924c77edc08d5cbbb4857c4693305a31bf401c5033af9826299a6638446e2ef55edaf478210f755b15dd78ee0d84231ee4eb512456905fe27a34be52f27e3ef067189dae3056ae894192ce2dabbbad23d2e9fc328ad668622ac29b429bae174ee1f02909803cdfc6223dd3c3f251ab053d2d4bdca53d849454ac629fb05c0d71e77882f9d0521325ba3eb41650874009b602cce103895bf3f26d5d64b3eb8e2011da4be320fc2e3076556dc8705f48cb101d9c850a9e6b4a1ac3113fbd78bce3e5f5919810ede09cab8cd2e8f868597669c469b6e922b3650a41fa74b49570349b5f291d50eda4e34e84229ea3ae03d8538fbf1b1227a2fb42186aa48922b05d18d403b1c0b27fe31379ea53a0253950e20356f820c4913045103945028d9b31434f9e136029432a923a9ddc39b1935725e66f94031125c3f4876ab57bc830231867fe0391177669d1307912e5679570f374be31ecbce42da0384431536c330b792301fdf07c6e4ce49c25685ffd03592eacd6b47d4d436587b61935896b7d9294f7c614a0ca33b241d596b0b152b1b91858699102bc1298a8a54f1f2e33d54f6fe428a2a085f1e5069192d72424145319984c0ae3b23e508e8d242d802a1dcda1df6ae1201553578fbf7e2b80cb0aee5625ea2955923b79e9f55e772d729e9da4b1e8b810af95209de6de52b3c47f0d079414915d18758821d656f2c7a2d49b8d0e48a4a0054380441d2158b55e6eb991b9da64fb9e8cf8957bb0a60afca9afb18be70510a8fc5c742ebc22b2748bca8b751b56597d83369d1a9ca8ed7110d622e454e780f00000000000000000000419cc293015ec812d5613049d0fb052dcfdc899d71eba13c571e92d7b121911fe971d0f738ea79b7d10f13e450e72b0d766174667ca2569a90a8322464b2f1269a8a7c074f89d48bc3c74f2d68df0313f165ed4022304bbc4f2a6f0e1ebf9f0094593e8a307170c6b8fc6a1c9997fa89df58775f159539bcbdd9941a1ac51374f006b6d4fd3795f424f7999b98e15b78297655372e64919ab32231134735164c3064806c56445479c3821063b03bc0de167df9f23bf6f420640d5470e9cfc200d55b062c122ddb369644e52353367d45383451cdabdb3606ebd11c60f553c3f51083aa6eff0ef14d4cba4e561be354d5ad84419abde8d40cd8366cb9685a89e93e70c14baffbe4d3ac650bf70458a16d03163015f87979e5559cdb0360f7d180017675ab9f340dd888217d024b090bb500ad40d3775dc845884f242d5f55c7710d50894c9a250b80d9706dce7ed47cc07b329d80bb494f6744f6f1aad5518ac1066d91e4dd8cc836463bb09537bb1ee592ede10b38aeee6d470060cdd38a81550b93e55828aa325d84aea13f512436347c43570139fb9c7215e9539bd0abec200a3fda5fe8177fa98191a1d5a58bf4a54441cc76b607c9b691f42c8e968456bd11d025f8170f50dca772081724edcd24f2b3d721120b5ff88b5ae8148777a5de050318f8b5bc615c6992747db03c72a26255c3a2c49f254afc124d2ce75f262207ff729b2846a019e55367d923c8bf38a48e090af1e719a0877da75410ad01d20368a70f5470986ddb287e2d42ea5baebf91cd26a9b2c9a9e6779b965dd08ee10019403aa82eed74fbe4507dd8af87a186838aa34cf67dabec28728ef526f1090f2f60d4a5157f4b53c407a8f6f83728ab0ed0f282116bd6a2a2f3c20f26b48a11c7593432257109a47bb55089ebca3aa243956b49cc94e62ed52b5ecd813d3805be134f28514037837d7f16f4936ba8d14ee724b00bb70e2473657fa40273e205177c81559a9f4aa3304cf7720bcad529252f606347a0f7a78dcbb790ac40bb11395b3332aeaec69bf5a321a7c2fa56c8570a7c9760791ed02657dc3249730a073348bbd325d9df18c01d02727b0627c8363c9003ac4d8ee3aaa79a33e135d603da7644071cbe17d3a2c7462c6c1762ad692815b6d36d8141096ed3b643bf4b03ca3a491166b8f7d02dc2b8926261db125e5317f790242990a3cc7200dc59e508af5fd92872dd1ced7d75abe3b70d206bd3a942b3fa85879dfe647bac897cbf0a1c640f79d146760904bce894e64d5103fe4960ca7cdb7078837172f2c2ef520024e83f2bccadcdb28451a4603395bfe8a332ad1d43e297da69c2e0ccbbcb0a0000000000000000000000056c56f028c1dcaec02bdf989e560e9fe7a766afea1ca8d5ad8bbfc5376a631a2338da10e4871584994a2453acd33331890fbca925cf91dc49fc4ed2ea0eb2af45565bc8ae1afe3789a70a928984a623d821ffea928a875151f3d4cc472624804c7fd57535d14e44fedd3e5188f1e704bddc045e38e7f3ffe21eca5499e6e97095ee3f19d5d8dc0ad3ca54306c9565595778c036f4600f901ff8ba944d1dd9eca8669e390d47c5225c6fd6fdaaf759fdb502ddc0294ec75789726108fb2034800890f0ac3e3eefb309d2bf4cf7e1db0221b197faa65d355a99bdb04a5af42cb7f7c495777299fc640aee0517a3af645dac065b630fade5b0f837a20f3179d2f037dc43fddb0517f55df6594be1909ab8792ea470c8fed9f25bcc92bc1ef6f900018d8be401f3364b2db70d7bcec8ff8999520b9598340db1efd1652080f1c2060065d6cf40a5708cdf847fe0f982e5c6189a1d13b22c98d06e74db26bf2f4726000efe8a8e65e9847d3b8559c0ea0a406d35a88df992315ad1a3c4566ef4f1a31292478cdf02b0f1106eeeb860fe391b5509420c8b64591f32e04947fd29c0aa066c382198de1430437afd7b6f8a65d2bc01c71b97eb19d02509f7eb5712b4e4101054deef216c3a028110b7df6d08f04ba040295a79b5d53df444bf93bd4837096daab0a5ef8064c6411c76959e07e0aa48752db7f8795a3d0be6a8f23adfd20871a489e4968066d1ad4f21c5ee8abd2728689dba8534011b9cd7d3c32ff5dc099d2a917e7346352cbaedce78c87ace9902e64a36a97ce908c8a7fce9c668c40909ea775b8791be758afb712809a53a3b2e378f539311f791d7c4b7039157c00af3e9f298101673fa2e83b503a75a8da5f5332983a8b96a9126bec16f6b06ee0325746716756f1ebf4691ff463a150ba9a30e2e94167567443690076c4b25e900f6030bcdebe043303cbc7be11c71e336eaf98f9c800f2c63ef1a294fea138510f9250e4c4f95bce3c2f0cc225148411abad318aa20446c9d41a7cec454e23301e533d713a1637e95befd692e45afb461356bd212da138833c92718a1dac8ea0c46e991a8ccce1eda49e4273621ece08a62038862a7f3cbfdb47b74f7e89e9607edb626ce15ad6e0bd081814b43302fdc0ec548d66222b730f7461665fe17cc0d5c9021da4d13ad35e5f2bbfb4a90562d4c1a7de92b4b4a9037e5ad80b717d5107c2fba43879855827d438badd572a83f90498b2f229e84f9bc35dedbd486b908061af77ece6d63ff50d36317873fcc2fd9137b7df804183b1278b6a2917ad00947c01f2cf7d74bfc624e6499e257c5ff36ebd4c2413312a9c21a5b5de185a61000000000000000000000f1a4fa0ea16bc04ee86155e819b2d6d05c2399e232fcd69a2305662165a15e2822af086644d96798686a9d7bc39762206e14924c7a66096e0748c0dbe7eaf039bb5266154113b38b3efec18a298505f3f5749907908686355f26dfd8d4278380cbc7ff16a53ed28b599353b6050520c06dae23df48a3a0eff99422b62cdc3393a36bed89d4eea81a074b8e28a3eb9c842834ba5eb1b490f87f9255622c68d365cce9aac44007f84834a9918e2b32bf06458bbc4d56d67a9283d69d81b6a372800bb0cb3c06b2f4d83270b7e6a26bcddb51f7233ce656e124bc043b45b7e3e12a9b826d703f0654d36677704b7c9466204f6e00a4deb929b1a6cd36067f688acec11829d6b86c16985c3f0de194f850184b1e3836f84f083f574d6ca1c87dfd0001162921d7701a687668a22d4ffe2a51d414cb7302c134619c9554f3f8a367560429d3c1c0f9621006ec5eb1376b6ec5109151292c0234fc97a13d969bf7735f0c0f58bd8e3ced89941726b5dbe59ad0e36645b06f74840f1b1f97c4d27b62d90921b8fd93cf0d1107e96a8d499972171b2de0916590b015908ad9d3532e97af0db3a6bc5dfdcdce00696e3c57e831de3b95683e4f60339d871e4444b1a63fd509314b25a8c92b33b7cf8ef791976879278c326845c03f8eccfd5eefcf66e43e05665756e7c15333173c1e3cc3621220a32357ab0838d26816e057f63408bdb80e835d0b7c3d50e69fe141f07bcd6c22795cde054ccf1edd797ec579735ab54d08671e717641ec7fd1bbcb67120d7451a1bddd4f7126b6292707b0829cca8830084a38689855ee5e9645ec82711d6309d8b95542529345742bcfa4ca701beec00f42383bbd6cf00874861932300eb9168118ecd4d50cc613d1c9eda4d98e634d01c49d2f8c10b641cc40ccf2bfe468de838e1e02e977f221d6d0875504ef9efe01b414ccb79e224bd741f88deec9e97a50f298dbf25d770a4738cd2ff4d0d6270531427715126b6c4b212cc573c84c63dfd40d9a2ca388ea0bd844b5570d03880550275ac3fa3d73ce3cf022787505f24b158e3d7f31f68f86cd3b5ea8cd7c190309ecf553a9bd40cc0aef78f08b4866936c67009ec34bd8c90988c80182d4660ebd8a3ee7c5f60e75558ebe514d6b62b3b6c2bc3b91d0d57c097c7e183289bc06b5382f1b6b28e3d9ff2752e0110eaeb993d55e19d47d54204e3049427602cd025e2581aba8fdf1507be84125265581ac43581fd6a2455776c6f9b09b2e49eb064a53c8ef095eb9ec3f2eaea0c1f71668702b1e295c98795920da2fd073c69e104e6348927ebe514bf42ff9d6901155e209d79a010c1fd952939aef51025caf110000000000000000000027f35e26dfa069d7b2e59c51390e83af3c537392b12a5933707ae76e2567029f50ba25fd8103fedc485801520eb000e3b386e4cec302ce5dcb1d40dc64d70013a8448335a41e880b4d4d0db315a7ab4cbc1a4e3b0d79cbdd4b53299c12591681528f64d968005d5eed662d650bb3a83599ec65013f30ea141cbd7d9ff296ad8dd6a2c85f58202da4a8e29e25c60b65a70efd1c66c29625762f4ed4353c74080542327ee6eed115d31aa8a779c7e14e7d700a91693279ba0821bcd96e7e7c1400d234e8ed82acdbc79910bf5ac12c8a827a430252574350c8b61fa1e5c55fc6b265f57654fec93d42bebd94a0b96b21e5e38556fe12da769866ff8f583144727f7384fa712eb6a4388f8dcb0ed64dbf7049466985ee0275e842b71a5790c8920001546cbe057aea8e6869b4ce505f0205aa02c7b0fb0ee563d1ba82d710a730f005742b014059819514b3d605725e41e8f38b687faef4405624c9961ecaffdc4a0f92ab246815e25ae3fec24c267a01f4bb53d5f8dc73fab70dbb615c4d94523c019f8b252f2a45904a7696538464e482b5bd46dc1bbc2dc3bfd595f0f28c65ff0e5a00c27fcc1bf934909759b0cb5ccfa81fae8aadfd3bd740590c04633f6a5800b3815a6b0fd5f2c9db2f30be22b8f2fd24e6c605b598c8eb996f5f42a75bf30ba658e73e54fc5d4c5f8ea51debdebd029dc6f683a8f43ff6d203dcaacfaf1b0de2eb1275b3d5e07f4e448c6a3e4878288397d1e6484d15ae9633c34d9becb607096b7373f3b0bf299def196d60e5e6eff16333822685e0c477a61f0299b58000e7cdf932d20bc39885823cb8b85e91d83562891fa473df746ff60f01aec25a1148a772bdfd3f2530f8624cd55777bd611e5d9047d78ae75d60b87b810513c40df5a4d8491f0db3b6d5fa4ff20270b1bcc6a16fc724cafbe53f1de99cc0ad890a2f0b0dd6b38da299c1ea1ebb321ff54c3fdb73666496ad0c19a7b426acceac10c65dde2a42cbf353d86a5081bc8c34099b1a908e903b919244682ba9e5f43506ec97f05c924891aa3a52c65a75a2de506f3481ab5bba18266d4593b3a597b0002bc326926db4415e6b187cfcfa444b0aa83bdf85c3f339da977a0f6981dc2609cabe79692606f7c9de82ed0a55dcbc4ef36538402cfb630cda6b8dd52eb24f0fd704007a84cb1d12b46d98e09c98c1d901cbacd0f54044febc5c790cbca8650e5b35f87cf837d566c31b5b068077ed5a26757e7db6615f4abb2b1d9f5043d70ed7090afff6d4b0a3a973dd0a54bd2629a3135609b7403707f963e12ec479950578654e7da6e1e2b3b6d2b33e1b47a30f9d5f81a30f43f0492640b363018347000000000000000000000074fbefe65266255b396a8beff10f681bba07e4a7634077b22badca6058683f9e2286fda30821d18752d932716cf31bfe4b264176a201822b71d2a1811fa3c5e19da4b2d075e1ced4366f41ca1f7fd01fb39c0654d692dfb8a6975c9171dbf680bcd02b729b5f9ed4e4fe75745048ddb1cf208fe822e64af44c8a8073cb7b91ce59d38a94b5bc754b2ee6405f532d1f07adafc61631926ae16c4ee5ea51e90c8ce994efad7faf3db92248988069d7165ab54469e40ce714cb775d32cebd168600d88b4baaeee984e05527ebbc5120fbd94e1a80275df818a7b0b58db89bcbbd589f0e77fb62688a1c7fb989e890489ea183b862a16381b37a39079247931d3684a778a53ee8590f739cada790d04395fda28a344a7e389c57a9b23fd9a2169300018fe171f5c2cd98197641204fb2cd4eff002f68e0a7f234f8262b9c3862fb5211c76ebddb0dc39da1509f6574ba6996440f4bed64448f0c2f67e8e8d67435ad07192780d1b4f7028f9259cffec068cf905e641feebd85b1ff3858ea0aca612d0750bfd8dd3a811014991cd93f970b7116debbcb0be38f0bc17d965a83e972a61261a58dd51285f8b6cc63ff8c36236bb70df8664df8fa241862c3d4faa2c7750178e7f94f8aa40b4110f7b299ec9f09a424c6dc3021a40fe393927712ee929912cc076326b75e5866f4c06868815578590cfa25068a70358c37756847b299f50473d6e8ca7b8e1e6c2708ca58daa93f9d3198471f9cc3dc32739ae35b3e2f4912e41c50f6bda721ca2b2d07621eefb677d0ea7af91a63fe9a8e3fd91da60194003a3fbcaa59c6962e37f8e5fcb2ecc56e3180b50f7bd306be0bc523d4132853020269b55c499dc017c74c8395aa30dc101c0f85270d29807970cb13d0f278fa079e27200263ac4458f399e23be762858f0e17652d0436fe35cc939c019f07540dddf0a1fa812f1b2746b863100872ad750bf0f4a6df92058fe6a11be55f568f08c1979fa60b6da738d9ce6d143de8d916c1728d3ddc0aad4144fee549e3154503b3e86111a9ed92da4bc1a4cf5646b6726245c9d2c57422954daafc620a256a117d0b586531cf1ca70670a3335cbbd47f8f6c1a66e194196f88c2cef3bd89df08dd0adef2de0dfa9ddb0b56da8ee99dfe62b55aaa05193a990f9c18b1364d750550f18612636db34dc543f0b273cbca8c78029f637d7c9fcd6e15048c4b0c6811645c267114fd399fcfe940d74487f7f1e5e870690f31323df660c47da937560743cddd601744885ef0fb36139ca2a297a94f801deec4f1d5f66f2e7b5b8de800bc52a28f63df71ddb1a2cd4835e49721f1c4354863abbf62aa95408f9a1f300600000000000000000000f807de01d6df4531d0f71c9e363fd721edebe2c450f333a838a5bf1fa584268bc02d9ca4983a24a68fa3d3dc79c9153a67b96f0784976b1881c7513ae688731803b432bc50dda91b8ea206a02b2bed9163fcf22ab374e5dc1702d95cc1844f0090b0b776e11a1a0719c2e967f78fe4041e207cef35459a4e521953bf09800f40c530447499a9c43fe9b3dc8b66563822675f55d482ac06e5dcf5e6f0688d531b61a132529a751946f405d42bba9be5c5dd09100b0c2458f5cc912339e532a7002efbca39e92c32b9cd407f989c64a5880c896d32b5bf93446163f3ba2367942802e5799c9919f02358954252d4464953ddf13a1715d60e302bf6daa58f4f7ade0e79bbe669d8dfef1c3d9544f4fb53743c5ec8f043dcd8f2288a853f5b4b84000116afa75b1c2563360e9e54a7b0aaf449197ae69f795e5d580431abf87ff6680b5ee6127a189200b772df244b485f952c0b2eb87c63cabfc70abea8b6309ac607e6c973426d1e825a05a44e013f99c4364f782fd46bae9dc5110d53900fdf6a1135182b9fff7dae5655d8f91e1b2936eb4dd54d8d02a5d72fa00364f3af251b0316f17ba5bb13a347f6dd2869f22c7d8073885400c73f41c16b5548a30bfe8b02f26567fd44dcfd41c6119cf4c627621cee0f84f5b3308d0acca022c759fa76121c4001c66b7841a21ac36ab63b0c49387b3319797fedc0d0729b5d518c76af06822e30e879cbe897f60a3c8732a3d3f6952d5a5ed5ab5132e69c0207e3f4be07172da78df37bd09904be550d1bb7ce11ea8665e1eabdf470272233927e12580093841e2df41edca9fa490ecae3b0fd6e1990c5721a8aebf1e3ac35608a80540ab72c9ca42359f7eeacb04dc3dad14290fa13614de6217a5656aba68deb66ce091ac8d3a51e1641e5799d4e0aa8e4bc0c94bfc58f305e3bab28e058f7e27a070607617daf67e531ffb1701747ccd153e36f189fcd9fd582e021f13040040e56109e07643ed8a05e1daf3c113065a17f615a3fdccbca88e4ac8e8c327d32f1e610bba8cbbe6d1190ec364b6d8a03886eb0fda5911b6b40074d4f092411813cb002b880ad4b89256e433bd1a7b51316e3d94a009acec7998fb7959532f8641db60073636e74b7b268ca7d91f35a00fd51e1e548307431105ae2cebf2bb31e780d085178e9079d89af04d6c68fae60b798cd2e1d6c6d7cb90df7c7edbed8f0906404ba7dac9bb9afe99efdfc777d851ae67bcda06d5f5273a67e3fd7c4f41157fd0ea9065ca6eea8727e80955bd1680ccc34f542904e9866074d39543d1caf81370949cf09c5381dc3b1151edc3c2b34cb46b5556f4d760136920cc5f6e1e61bac0800000000000000000000cffeafdb8ece08041056395999a1cc7064c786867a749adea2af1e82dcf264618d44a3bfab30bfa23ad86079eaf537bc564e2c171ea4073b26b3f62a29ccc38e94861532e0d50e14b30e62892435117720e18b261b811f56b618cec4cd59950091f9d792c5666dc2ee1161875104cb09c5bed800c3bee962dc2ffb97888570fec02a921b26d4a4a571f211aa1b378ce6c8184bf09f5e86c21f514613b1ecd892fac1cecab53e8db2d44f38b372e4d39520987dddc2f7a62c752beddaf25265009e8035a6cd50fbb89bffadcc8fe533163eea0d6c8d52749eaea624c37c712114dca746d35d38ee5481a7365eba13c4b8c6a8e19ff75463b1dcd6dec80c6a56fbb95488f7b94730cb59cbc779d80996fedd2c5e9bff42085f2c5bf0c5d51fa78001b6176f9fa82961c7c7e30d492da1518660ca98b230080f04fa6d29609f4a28098feb2c6fa3f176d232dd8f8802b9eab253958941462f1981a1e2e383a963a5093a6524395e26d9913c88847cb7bf7fb4450056ffc71a15e640dd3b794c12af088ef440f356f583686c2719dbaebf5856efb5796addc9af6c0b418c8ec628df0f113ef2da27ab78da2cc05f0be62419ceede4f1815690ccf8a2e56e4b2485f100f2c877d2d11f5416d519395e5913a16f2645f612887e165b5070f50588a0af0a55aed24bbea01f780a9bb4db701f6d36346d5fc2e2a58bb61d5e402fe85f93126eb7d18d5d367606698c76186280c2ec7eadb904ba0c28cd73a132bf61e6241078bcb1fcabb95bf94045d90fcb5d91baae411a9b5b8ce7d8470cb41b02e7b8019ce36a7cf3a1b942b841858f141d09103cb4f62deba616239e7f91aca753460b6dff28b61fe7388bddb174e275bcc1e0ecdcae49536ebf70582a01578cb00b08c9436162f5c158b1db87f424baca033497af4c8c14151127e4f8b78d8680860ca5d7091c768f58679153c2d6686d7526722c6a52e74199756b8545b695edf310fcc391607c0928c1956ab4cf21679d203a1f0d286cca828693b5cbc09cc5de06419fb127e7abfe4add3e26fe90435af10bf57834e4c73359eb33b6f3163b4b11865b70987e2de41b7100067c0a98ced42129f3e34ae77d6e3b51977f92585c0fefb01ddc8c192e9571ae5e98b3c7940a436559c998c8780fac929b42f854810b34ca88e3682504a2c700d0b05a12ccc1297623618df6329a296958d54b6577045e9f032c99a6a33164cd0239837ffb5ec4701701d23186589a356c5d443148057f79d196ae16ceafcd87fc417774b169b5c8adbf7e5009d91d56ca99b6513801de9b75328b4e8768464a0efb2c5571eb3aa3a8c473fa1d752ffd3c5dbd4cbf0f000000000000000000003fb1a4041af282302c38eb924c7beedf2e049448584d1086b3d951ee3670fd7f7684afc04ca2e39264207241a316a38a965ccfe3fffabe715e34c905699015b5d16a4aea610a349c259fcd2d74b4e5e5b49502827255b1075ec01d3a970c1a00a8a2ad9f6bfdc33578f741cd8edf5a94657a4f7b380f7b37ab19fd33095912055d4e05b815219b464d56a001b6f32fe5b8a4993f20e630e7c699ecaa43f73a9ad36b667907052d5b228cf23561affbc32327a84a076e2f3136afc982b7d26080a569538911f79be9a4e349cdee2a1ca04ccf4b6e224b9d8c2e7f85fe2caebce02b4015b5f619c98e5319a38e030211067c67aaad671899716b1f6d6b4cced1502ec4f6fb2bcdedafc493e2403165f8e5063df34d1ba40bdfe8a2ebaaf3136f8001ec5f3b18bca43677d5f6318b361ac364ccab7b4dd397a172e1e87fba491b82123424658ab033217179b9c2bf8de2bb2b2569ff7eec5e9f51f5a72e4ef998f311e271484a7d39d7f29cea438f87560fc2cee6a339b3591243b6540ab14790af02eb1e61807a4baa0fcc0257732a37e202bc9641e81c3265d3edd136888a799607540ecb9c27003925a4ecd100c42c9fd7f260de68e5cdaeea950d5282d7b9560db1cdd0c2c08c3940b4d8a176d07d368c16d10e377d5184e8fc6e130e2bebe406db5aacec6b3fa67126b6920ddf99254f8ecaf4879d8b2bb2e64c891f4afec3023cca5badde3a55f92f87ffdc3e7fff2cbaf52eb39b65f8b4a3a903bc4267bd06d8f720b524c0fa47500da4a7d2d3e5565f8ca43569f3dd0168ed5e3c555d8601969913a4af2bc6947bbd044c6fab551f3c94ef8df98ef6ac1f949607d0e630007085a5393568c0622d50aa72d0f12742a928b13dbe49bc3df4f74f9d18cd70049989f8b43e4c3f7c0cbcf7d596f1d43045f9d7f1f3c4568813d5fa488ff0b9071459f7ca6f14ab0bba389c8a699b6c6f8047fe0eb918e844ae718c018c8ff802cc040803f24a46b923b4e22984cce17c99475acb20bfa6b214ea2b781574bf0a8358384288c36bdeead2d8ae57a962ca988e49b8d19905bf4c4ef1f13d31bb0868488f3e9c6533748f2dbf54628aeddd94e8919f7698c89f1fc24341e078d31015d39a615fbc3457424ecdd6ae043bf028fc0d2f4b6125cf3f3de03d6a614d0bff9ecbe27f111154758a80aa7433ef14d3f4c5a4f04de6470a7df2c2f0adb610220d30fcf0ad119fc68985b997dfcbf0ab0ebe63c17ee67ca730ed5d0854a90cfc7ff4c9fe5681b6de6b4a5280ebcf8b10ed3a4f30357da571796617bae08604ff9913b1d5a8e66acd7ae8f85b0105480638544d5e2106d3d8cbd023449c5b0a00000000000000000000fdc7406955990540c3695887d2aae069dcf207014b2a4f0ee8fbf149a9cee503257a0f51444b38a7b8812ea2239755ebad7076971246ee9ba9e1588876eac7d9303c81b09567145ce866ccad3b655b4e073dd97fd9591042bfa5441e6c0ba1803221d89939ea52cb2aec978e05b28b3fe6bf096cb973fcfcfa437a1bf1691a521e934463bfa72031579c6563d9dd8263a2724df0639fb2f00ccfa22275fb2fcf0ecc6da6b022029babfba902d6fcbd576132fa85f7016caa0cc12d481169bd00cc4cdedddcaf7afd879505f24e9b5e4ff77cdddb78deeab21a783a3d2a78b209092e960ededfdcad5b36933da03ef3930a85debc12912e8524386ad40ab2a82a6162e212ea05b5b7c9372fdaa1410c0a2db32e63138fb9d277fc803fbbef490001b139c95c82cf1fae4671d95292e617fc0bee0235679663557dd274acaa390f09022f2e49f0cc43ab37a733783a9980baed078833cb1ea6cea099545b582c310e147795e438d32f77ba6fa5cb84028bbd9f047633085b49495e6b12dae4ffb410b3b11f78956bf8ff57e0fd2fef0415e1eb59aeb882d3e16240be1661bd6a3a0e12fc78516f9ab4cbfe81facd457905c8209a0306715a6d21bfcd18507b9b8600f7904affae7c691379a1917b54fccab0e96f327fb95aa4e280c2e058f2705b0010f156aa88fdb69492bd90ba5cb60089624ab7d8956d5ecb52c22122409c8810a636da9a8378fc0915ae3289c494c9ec8f9b733a5703e5c42e2c55ac45beb90aa2499c015f8b5c4a3c512f820f5cbadb6a2d0958173154173a076768aa544a086257f0457573bcff5daada24459791debe6ca6b85c0b5fda0d0ba2db39fccd0a400a57a750b80cb7c153532726c028b07aef817b41c0c231462f60f1cf5ef811794b4a518df1fc22baef85327ec0ea3c691dd7a9d4c1269c2f6c23b897279109443eeb8f30e6c647bc3dcd79d389287987a44dc7bf8291f34dc9bb33b01f3410ce11c87949111f10e37f8a794d76a6ff864e4b4b9c3f710cde25fcbd7b1da211274bea8c8c17de2b7f1274bfa0580e0753294b4bd275622fe93d73c349e27010693d2714f4187fd8b442480494de1c04fec1f7fb9ee6d88aa976a22e5543ce08452c3a6315fb31410fe6eafe2094f079a5d2766d9e9c91c216c68e6eb8f3fe072484d1e3b35527e0194123f60e2cb141e1adec24c1a40276fca5318e7f0997034417c274e7e06516e823f3866426000dfd800ed63ffe35c0b8b14c552babec07e759b101cc614bd46b74bc6c0bec85cb78da6344d359347ae71a4004daa3b30209b7f7aa73b313a6eb635bbf9313f33d970fda2633afd684bc5275a7bce585040000000000000000000036fbf639f802151110c80b6fe56d992afc2ee74e77a69b459aafcbe96a8340e7a1c8ac4119421460c00dc81d985e8e99161eb63d5ea995322458e9b490c9aeadd4429ffc73bf6c1c4cc88ba37a62f16c7b46eac5c37d7cf42f4c5bee1b219500959bd0f154caa573e409ce0a8a53072c4936bb94c2afe2ff71e67a684f7648a383f4742024d4af9912e345ea4411cdf27a1513a9beb0dc52f106cfe06a8bcfc6e595d3060d5e78a92925a7a54ba06e43dbb07e46d51e317cf82914ad5fc81600640942ee77247ca7350262cea346a1c9de7feb62c2eb3c39232313ad6b6948a7adea5c339750c088ddfeba975b8d47b606d8efb1a34491772084247892f550daefa4be0d040dff412c48996ded116058f2e9280c93a55b85afa0b8f4644b8400014bbedad3d7cbe692d689d23474c9d6e20cfa6732157f62f88a0dd0f0d6c0c508a4863239855cbe301c4d7812c9079848ddabfd0552bd65919fae3bbffd3b58111d85c1ee435b93af440044c103de5dd428f4e7ef4c5331287bc02b443a352f003eefc2dd3b0c87b0553d5af2821aed770390a22957cb693d0c535296e36a810859e508dd8dc911ddd26021fd08c9c93192da88ff99a3c7db1502ee3428b49512dbf88fe12ff32786fbe348e3d91037d55d0806b529ce552b5c1ed60d74587902d8d969d377f0af0d5d7146027eef4c783c51756c5e35ddb0bd8e0580482ba407f492f516c0efc131e6c4a51bfcfc9b7c7804a037b769209cc67e1ea5a06ee70a4744592e14cf6d8d991434b7441a39c43aabe3486a9e5213f2349d7d19f4640be5bad29da2e489b8402aeb38ce74bd0c873f5cb1b9acf00c2b91d39e419e3b0e08cf3567ef29da27bebfff746432d85610c11ec0376a7fbab2138c1a0bdf7e09cae288c2f63a40342e67ef4084c1be265ac2fc21e24a6e8b0670545f1e35fe0ee45e2a47bd31d87d6cdc5105ea322bc089bad21b0ebf14089b27dc656cc7ef06daf09f5ede494193c5cd9464ca8a626be9efeb203ac5ccdc0f81b8a0ecc696010d10ae7d04660d3fde15ac92b5ddc04681e2f34f9c5be75fb6743a12d940220c264563409e2b086960fe86ed9ac27e899813711800fd066d6e71101eb9f1270653b25959855042af3aa7e6644c147e3d815cbafd7cbd67e70f64e1220d644305f8aab48a82c5836bcb79e1205ee022c988667d1b90d1ef7a62b21d89c922e406b751d7fb1fe4711d8f0f52f558883c8da2292de3b97fc4508290f4bdc3bef80d601db9b0f4e9047f580d9d590b578a741329ec90d9e3c90b84118af92eb82c0dda1a96c922d8a2d03add035974d40b6120086f7f7424469b53da9e5fd35ef90b0000000000000000000036caf6412c7fcb16d324ad36f208e46eba725d840fc412f1294f29815f30a9f4d09be9799cc1eabc643214916575ea0342d3d522e74e77bacfd3c5d447b3c7bd751e0e66695be6f3db5b9562692e5c51736ae1fe19828b22b896aa97db6de380e1a38756b178d2eabf6330c31d3e8090f36a8a9a77ec6b62f5cc5c179d3e1c68447659b737fb0eca441642d39f0fff700b949e7e7def7015aef42b6abfb475bb1075092c0e64740b725a472e276dc9b8f213ad09cb71a3d6ddacb4ab1ab9198181425d83be79724cc9cf8cb509d8a0d3079030ffcda8c8ea8badc99603831866b92a0f9b03063808e3b8c359a0969a51eebec0380b110fd8968433d6db6874000b47d607be3978f22870d6971b142cee301983704806623c539d3ab06ebe3a8001b03053384c8754a21438143235981db4036cfb9b867e344ca727f6e93ed4b609721f39e0c8e6bb42faafc84a66faf50cfad3985d3040065d2f9e027c9f42bf0c646682fb70910de972c2882d76dcd0c752be113e229335239620b6d3eac96201a10d9ee2fb8c113320b9b3a17d369a0a67b73557c22e4eecd5fd4edca41cf603e22cbef51700d8667155a7032649e526cdd54df40f4ece08a1a9218e1f156b0d62ff19d30ae4a2d78a76da6cd53e6658060e793064029d184239b94421c3aa03b68d11a30d0e89d80f463fd43bdc57dc7f3412069d1d5ee50a30a2fedade4e05f496a7fe0ec7df3e11ad18493080a390ef30343f3fd81204475e525791b3ba0f0c7178a6c55788f9bcfd56c6a777769765417690fc4e7e594213d87b34792c015ecf07f964f910b62f3933e8b30203d6a8230775e4183954ac640a6d76c82b0606fecc379a533960ea3bff4e35be7147520ffa8c38a5822e147e25735be2640aaf7ea7d75c17fb5f627f914e3bd0955ebd7ac0ce77a6c4080be47e40a95a0f05420675babf667feaab40000d0b9cb5db343cbfa67c3d4b63ae5c4ef429287308b53441295ec88f97277d5b1bb0e9bca8159d90e6d41040738d9d60781632b30b9d222651f19cb4d5f4502ce26a284b8bbf4ae86fa070e93ba4a57769ac7efb0aae684c9c44f0798935c2d3e799b122556f667228cb4af064122d92b52b73ae06afa76734c5e788aafc4faa2bf915d88127d601496a68679f3bcba5d314c27b0d2045162fca40fbb988f01c071c31373d07108eb1f87c404ae2c4d195506aea0e4abc7a44610282f0b4bca6db8a7dfa0e1cc105bc043ff23aa55374ba1f94fc08a6256f5852d75d83f0e47700aa54e46c7a2e29e899cf33604ed4bf55534b6b12b7fa2371961ec0a01acafbf94542a7c059de43d596e65523af965aa8d3a98e12000000000000000000009533473c6c5fce3c9c60709ba529f35c3b35e868c9a29604fc23ffadd884c5cdd17e7fc974aaf7e1d474937239386a325517cf8b626d5ef0801774cb8e65456dfa257268db18f333eac5a733a361e73a0eecf99417e83e7ac67cbd156cbbf280adb343cd675fbb68756a6208b6d5901e4a0d533008fd0f11b3a433c7305783153246b368744670deb4ab899d2e91e545815458337167dfedcfd580666d7ae2ea9ce5c64d04e9154f2d619697b288e234b0e2f829f5388815bbe893d490dc2f009da4b356230b797b4015624fda6c317c09f52b8a786b26ec26c214eb9c357b15bdda0a2f5b84eafe42827d0537577a1d1274b06c18b218e15a4e0313faaaac92eb6e8a97bb25a03bdd8339a0d77acb9d8820dde16d1718516aa0e4efb5130a8001e9ae9c5119c60bd7bdbcbe1a6592998b089768d0f68c54f7f8be3e32ab025502a92ead7ee323e29e541dbe60c87a39c8bf5cdcda1feadc4e4c28cc080811f505d60454d8a9e0e3ea116bea8ffe362eb3b32a93348fd0b1e79d20c19fd4c5bd01a5067c36bf892eb3692a6cd71b1ded659477c3a4dfea3b20c115992eb2f70f10ef2b758fd001c56ef2d8850c4921d5171d266ac2cb6de48b863185ca0a21330c33bfa6e8c34a773cfe824dfb3b070de667160f412cacd2423e448de4a48e9f0b5c5cc368a48092691c8b4375fbc0f4e02f72595ee301cfb42201fe208ffdec052b78b44b32cfe7a9b8236694d938f6fd2f234f2f2f45e34d5cdf0176dccb640fe27af33966c2501d4594a0c5b6f75ea918ff6943482d3740650deef8354c280b292a27ba56b727e664be7f212155820f139b1ed4c725186e12e09e69b25a480993bc9263a18e441baeb4b7157035cd2bc5be60cab018a828a40950c349269d0cda795d7d138c2b8167c1f1c8c4a5d0d581f50e57f0f13cedf47328eeace39f0fe97a03c3656c99de73599619097e584b9117f76cafc891cbfebd5e60845d5c076ea4d8ae78f93af1ed23b9b7b39aa511c59c2bb6dea9660504c9bd919aca3912d159a62eb96541682d0941c4f9ff25b168860bd2d55624442731614cb246810d20a3f3e1504417043bc13ff52314bd4ccfe552ac999de0e5c36adc8bfed4e50390b390d7563046163fa618d7735afaf6aee0e281a3e43423023dc14d88495604e3a41f3598147d6e00d251278fe73636b58048a42d4fc86e7fb22ae37fe47903a9c2d70f2ecc81e6b73a6fbed755695636f2d27b1855164756a799cd7313b20d7e97e24b04d048ef56cf1d7620032556a3980d04c22ccd9775bcfde954a0550f4de0f51996a4ae65affea403d86ee6960072812e5c05bec7a069cdded8ac710c0000000000000000000006b09ff4a46cfc8451270d9c9ad869fc84f1c99c10ed2b573c06ba6f1d6f58906b1fb8bc4b2e8a58b64ffe96e9bcd1eefd40bf1ea6fcac64e097b4c953e9332baa0db0e5e2fae48840efdde7d12022ecc29f8d84c13838e434d379a31b95008104f95a094a4e7ef3e83b3892b5efff8d502b8863bcc482dbbbebd3d9f05265e1c963c35f0232c5a219430700f7308f4f70dedbc9d5692978aa17a3f0e705de481b36dd7fb787068e11e55b9fc09408f1693de3aa6dc51072cc597c74cb13fe8083b88d0b2e2612af36cbdedb213b68a5321d4ec0801e41ad29f3ed549f2d275d49a0383b56d567531ac899aa9d546684539b6b27fbc932e40bbf4dc265c0f88927c5b9d767dde3a13d04fe1679a6afa8c923d59380e76f6f08be51ba8aa1de8001eef075e5431d0f0ec261abf63b22cd623b64cc0376ce73b5b0fdc8eefe2e67063344fcf6b251aecb5246ab89663fc78690f6a37a224d82918129694c5179f50671c6e9f3a13d6e1bce8fb0e967d1fe8ec33ba96905c45686d3f533181fd2af005ce261a965e4884c74fff987bcf97aa9ef36bf6fba1787c74511d68060e9aa0cd4bb2a2affccbdd417845606f9e60e7f57621bc6d4149c21fdd82c42583fd304a4fb70a625aa8f01dfc02a39c0f62fa7aa7fa8bbdb1013745e1f65bc580e100a06a246923d5c554dee10648bb297275a69505c1bce59f5fc97ea3a1fd37ead0e7abab3ede772371fe09701d7994542813b6b99b953c083668013b734b225410a9fe15e17362b64263755512c0c362b5ea2265e9644280c6a1c45b2978d4d760f4e8b2364449171a525f56456071bdc518b1012722ce29a860c534c2a6f2a3a12a0186cb2cf8179cf628b8e50b18fac11d17b267a6720c5daa94362e1341cac05ae3cdef9a722dc521386e830a5cf8b4af2d72a38b7f98c5aa6d9c9b3fc4805013d9878f96f1f6256968d16ce69b8ec36f2ed8033a5cc40a1d12d9fa38ab4100549d83e10417bc9161ac953191b89c12021135f559555d90bf58401be8b3fe707fc67025d245aeb6040796868a96bb11b498924ae512ce8af4c66170a08d541081be09b9bf9aefabd931b8ece42260a517fbab2740276de1a47d6ed8852dc740968a4e1936858631151a1ddc205dced32663d6d9ccfb9cf4b45bd0d5a31fe09012cf611e3211891150baeff6c754ee31d98d8e44b1670e765b11bd3af8233e8044d06baf296e42ab01f6e4575e70f15d7feb198cfd7e7cd58220a8b52796c5509652c4336d9a862abf5edfa25dcb545c118a71ecf68b6fa9cf8f0523075131d0d1b649b8541dfd69aa5bd030e013e05cf17ad7268d9b382c0bd3a78c5a088d30900000000000000000000f8f48311e88b65277bbb77bda2bcea911980e1412144757e33c4665925fced70b21d3bde730fcc67b6e792596dc7086a14638f42232829da022bbb8338fdacfaa3c8352ce820fe49fffc4ccb079d30cf512377355b1043065b8289e236591401ab3a88252e5987f4239b69dd3e6f380df022986b2010e1ebcb290d217a8c789234f20fcd67e4bfe0a9aa60511cc0367f6eed974252dfcaa015dd7bbf0aafa3a2129bb9d408833cf7b6cc75ff6ac4d7dbd8e0708fa428dffcb448fc1be5c8ec80c61ac188bae19fe3a8f0a441f9bab3960be08682c216b78265c7bec78ccd3080a0778076f3724ca330071d4a1aaabed5c261c662b8f8169ffeae473d13cb3812a271939b554628b1409094c65d6586930824a7c9befa5f11a65d4cf940a318800113f7d5c432846b40414b8e4b4a5a234b05bc8554d7dceb18c27d4e93055f12054b9ce6e22f483159c9a3139a96de9a1d66b377addd87f1592dba34aeb1ca650447c4381bfc2ae4ecd52d4cda607375db36b20ecdec9b4cb4dced899bf82fc503ce7fb2e1918ca07b82f4aaf0c5fd6f05d6067c90a99e4c63acc43abd1e7d200fbe0c1abebd1ce5284f7b040a9e6429be52b5e29ef6684577458a0ca44649050a73a0f3019a0612ef5228192ec6e6728a4bb77122698401086520d46716b65207e4bee6644ec7812af14ff916352c5b479349b610868370f255a624899b10e90a454520167bd3de9124c73fa3afe40b581b0dc00f1f91a9967c4a1376408b37020d881bd5a0441b374a0706471fd09e398830834e086eb45c8063d7a6fab03001b3d1f29d18d81421ec1ab3e2b0b0d6f1668b59fe27ff64cc3c2290bde9ed8507c78c9cd2ddc81d7e840b7e91e826a6b551453c254d8b098f282a30610226940a4c6c23ec64f67b8ddcbc43dba6c64bdf1f78bf132811a9f295660f444b5d010bca0de753e8a4cedbbff79234efb945e2f903c4a2ebbd6a3aa05b8e44e5d15d0669a663b0111fd5d687ecede3da311873a0212c914242abe61f40558a33b3e108eac03e32b43f099ffa7a40827ab539872121e62f38716d28ea23b1d9e141de061fc278ec482fa5c4bca1201d85b9d761422fa2a751b8102cc90368f60288060ccbdd3cdc48ed22ad5aed480b9d97aa100a896ae01356871f3d95d19f13a7fb075dbea0e5f3aa270bf5cb865516ac3f6122bb624b45e2f0e3014e321c5ebae60d26da1a0a23438a2696c579135fbb120f5da190df4865b0b7428e42999686f40374d3c2dc04f86f49dfd1ac7e29a867da8679da85cb7254e14593321e074a9b12513416d5af71b76aaa3462c029a119e254d18125fd66e379e61d8285d23f180000000000000000000000daf55520b182ad1c34574b6708d96e05234f4b4313e8cddad038db0c8b2931acc0ba3cf6f48b02f15f1174be108dd24c86d5fd2640c237a2b566a0874ff767c781ed36b701107c5ba95ee754ec70d286c4d314fe55bdc453cd4860dc4d8b4800b6fdfaf1af95a944be511f3d2ebbf8032dda5be8ff82b942091a0cfa7d823ade969de0c6889649c0762794a3cfddbf87f56b6a7205b2a364ae2e46b5654db82c48fa01a101c90efcb726a03719cebb79418903043dba95bd3458a0490f32d78048932c5641a1c138352d3838d94174f8ae2f51f5ffc11dbc9fae371bd76928912d6945b62895e5eea0dd359023dec8796ff8b402b8af8828222c436cadd0ab33c51f5dddb268500efbfe127a6a1b0c9fd40add60aa5e749d82259a291ff664000163b0921ef525e1585f64cf1fad16e2dfe3476a035026160a3518d4d8b30ea400766df96e22e0ee5431a46f709d8f5b605d1828adfdd4a76cca09b3ec0058000e78723d108e8a0c764f683e0b285190135d45c31a7b5b9c189a688f7a81c30011844a16e9fc1b4105b118cae134f26a9423124ad7857f2463c2288ca846deed022604351977aa2867899d664f3056fb14498189bbf0f78db5dd67f5d95197e002c0ee5a6140a7ab818bee24c5c4275d111f9a855ead552885ab03462710224f04b1a56831f49732eed3e8cc3e156fd26cc2533b618c70a04c32dc5da042745901e4208e39641a722e72360a81c629af34b3e39f99c1301c5b6434d9d15748810e1496e78ac1a8255b2d427edb1e0e95d4bafd6a8cf0a3f283c83607e34283420fc9729583d48160385ce706b906d53d6c506306890b9e3dcc81450f5d53f7ce08d50db4eb0538962328fc4c55a8ec016af05cf6712a432df858d34d6584f9c30a76057b7bbd369be092e9b397747a0bd3321f6d442cb66d8d2d1f093c2fa6680a75b20eefd1d0a56182e943ba05aad9404c40e2a8c064b41667569051626fe40ba5b77965c454bdb2ddd500a442d038d590eac63f36345cc3d91903b3fd1c5709ac8c16ef20c790272adde7c1fcc89f8f4d62b2ac3bf60054ebd9d26adcb329047251e568f4a3405ef2c1ad3ff9cf86a668bbc832827ed1dbcdf1d0e72348ed0142faeead0a69f51bbbf8ba7cc3d66ef9a269de0804bebe711621a6f273446b128a71e75d42c4188b12e15b4ec8e67625f92bee86f2b54aaedca2cc91dadb1f02cdafa777e5e8c9779c2c0d20d40514e6d6cd1dee118e83ce175f30af1c5ca708716e13468f6261c9c1d29f0014192e78f5b8888cc7fb2fcd70f15de87d28020177601efd59b8b9cb83ac6b5acbc6739acbea917c5242cfe8df481ee432588e01000000000000000000001ee2a5c119d5a4abab05068dd38478e76d03d65ac6fbf2430a431109f8b28339afd054eeceff21674a7cfec852d1069a2ef5aa90aa150bfa73338cd42eae5cca9df189ffff2bcb8cefc164ee46df47ecd0dceac6eb05cc2572b8fdefd3c9b700053c9b2a9e873b48c0dda32a30873154c6a032e812d8a64c28fb8adc2d78beae57ecd35999fe8fb45a38a4c1850a221fc507c37b403299b30ce72a46fef7aae9e06994b54f72abf44bd347647aad054be5d2ab1a27c750f4240c61c33b6c8a80f546beb6512986fb9bffee0fcc9943d94b3e1e61f45e25f12f770de3c98bbc2b8e220fb5feef61734ea5073da9c9c6f3c4ca43340f4e976c8be7c097a52332cf9d3d5c6ee9feaa2e9a1f61cf706f45e97e9147fabc9dbb93bf10b1eeec16fe8001db0eb904e42cc73db81ae24accdd1f11fee67c9d57280ed71fa2ff5a2f34e10642390568d8f88f6184a0996cbf1e773d70032da99e1f43d5c54849cc0682651026b8e4bc0d0e6fd01d51cf2bbd912a3d8cc619a003ae2e0d5f86f1c78e9b640674c93056fea9232d9506045101ca7ab03d43e4b163fcfff451fbfd572298b7013c43dddb95fd69e820c12797cdf8820972567e467d6dfc0fd46f78a90a2931050120e5fe2b84fab194f8a671b10ac263efd647d0d2c3f37cb6be33ba165b8212601e9674ba75bfae7641ab97a145c582bc711fc6e035e99b373c997fd468c108a84b8028e9d18a4d1a26a7bf5a55b4702530ec83ba672d950f48a6914822ce018140d0b99065fa1d7999ec86178f141eebefa048b038517ba84e873cc0d8ef02793e31722709ef78c01df1b2bbd8f3421789059d610dc592bfa33035e07cf0096b6f73280cee61246642d9c5ee2bc6134545188c870ce0ac2c1bfcd6945def04a17d8b991b869e0d7ee44fdb719d33c6a5493135b1e567e5dd21c93ed60c710f49d2aea406da0a07de0643345ad4ba3d26febad201f127712a107e19da1f5a05b88b37ae1e421acea3b904506b5e748ea6ee59be75022590237fc998d819340eb14c29273ce6c82c7254d39072c27fd2f85f5c3dadf3900a28231c2d122652062086d5b088bd280d6cab41823fc83d959818684f2ce4f2fe179d29c08da074121159c3f958fb84c71898b347a48541e3f12c0c9eb0d24827add91bcb2592860b4613c537a0fc2725e58a80279cdd09139058efcd926bc5bbdee282f0608b0b07f3422f3ac2d2948f5dbb12c2403a5508aadb47cfec3cbd2d523df0a44e2fbe0de4b0f1d74cd88d58179e585c376702e7e1a5a3676d3f1d405060fc01e3f6f8008be6429c34aa4d8368e603479383ee687659461cf1b361cf50dea00b449aa305000000000000000000000de6ea23d57aaed596fcf5042c674ca4b3a0a514b5864bed946abddc32dab4749dc6eeb59413cf9e5fbf8abec53461df7fc35d504225ef253089e42adb17b13110314dbb4063183d0c04f9504c6ef47c3eb4f7a636acd790e19d3e6257129c80a4e864a9ecf8553f013fe107813fcd8771a86a1aa2b1a2ab87b808acd794944a49318e259bf3722bb58fb5139fc5eace69e0d0b75f522c21801bbc2c1cf15943dd390385d8f40e7b6ef597cdf92c2a1b383e561a02e0c5b3217499543b17040024b79ffe5763b2f42bffc3c7ecec15b8e43438d49fe25954e39dfa906ba7543788728c209cf10ded1a89c9133fa613f370700cdb1ec91a29dc75b6f05700a5fc0eee6e0e02797f8f96f0fc4bb847f6858235b991d005707db3d2c65a2b6d0e0001e3267b2f9ab8ca377edc08344351800ba3eac77850a715cf9589c2fe5dd3600e434f4d6ccc16c35e3ff115ac726207026299d14061e54c6055d3dd87fed96f036455e33ce81dcb3536c125dc78ecb8d4a9500f54872333f974c146b27ab9b50e7d35a243bdb8da02b9bab67329ddc2d2f2d4c90875f34be419625b22ab02ef0c3fb6a931a85b0662b9dbaa823d64329bf1363bff57f7ada252db8a94de195901a5af900c17cd9d490eb720f9766b041df2fc303b63045bfb7875144b1e40cc084632f2336c7897c0967d77ba8f3cb1576724b7c5c1a7bfe6a5c602debe4b5b01da67658bfe8fb2292d2cfc188a3556f7277861104773fe8a9126255345c0250f6a10bb8432f74b7b111f7790b52fd07a521479229ac55b44ce0c1c01185ae30c3e32db4ccb2e58de737b1f0a0d182472c7467737eded5544b4a4b075ba07b8068cc6925fcc901ed92017a6a4c3534ce49d5d0b532ecc5744f0d50d0e98f82d00ec51686935d54a7245ea97034e22dbe5d169076d0651a2040932a35d2b7ee002b8976965a6937ab9e4a95fb12d50262e5ca560386d552b776ced858e183051039ccf39dcbe8a24ee3addb20291894ef879d9e88a509ae1149a7777176bc12504387dcf272f87661c77bd85eb318536814b2816c10b44abaa7aa1b52088793d013ab63aa96084252c4415cc4e38507cab3b5579b58e6be22f180ec47dbeb4f40a31c7dbe163d4bbcdc7710ac5fd4d2635b3226d90b40631202842859bb0b463111c437e098b0ca9a18fa928839b67f86d4cbbaa612d9779bf71d5f7841b59fb0774bb214262718975a984a4137ee0068f329ddf47214b9cd8cfb037d0760e2809a29ba8d3e9d85c9c6b21238f2f3399b54e1e39b6da1021d7cbb2ab1915c49500131444f4a83fc1cf06d77b76ec9b99c9872f7a2790c546b8cff77eaa0f0c281000000000000000000000a7fd8877f31900a62c2711467da4ff28ab119849baf03a9e662b12a71ac2fc1f0e2dcd233794e212b64243c2237d8a6bd7cc1b1b234bde292b04d14ced64a74998e964aea79c1ff1b75aae20823a5a97706285be5f50098480fd4911101b5400c6dea2214bdfb46b9e1dd4a854d1f5fc8280a81e312901d56ae84b94c8f98ca5892ea8b28bc624069739b66427248e498e5e4eaa1b47c8f893fa422f434ab76a328d2ac30ee72ed65fffee3c553b44ebb2e12c1e765f56292f975dda1d920880fff76872f6ae802415fc7830a143d165e05b40c57077083e438be64b1f96fe27e51c2e036544401f2f0dfa3f0a0dbc17874e90d879a07083f2f6258cc56cd6ce7f684dc5eee9e1de07b43c713a924cd9f102a26781025b743f3a4ca8e3b5ce8001"
    ];
    let response: <CurrentNetwork as Network>::TransactionID =
        rpc_client.request("sendtransaction", params).await.expect("Invalid response");

    // Check the transaction id.
    assert_eq!(
        response.to_string(),
        "at1yh7l65ege8kgzx5fsyuwldtsyk6k73m95pf7cr5tlqt7s2yvpcyssemtwd"
    );
}

#[tokio::test]
async fn test_get_memory_pool() {
    let mut rng = ChaChaRng::seed_from_u64(123456789);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<CurrentNetwork, Client<CurrentNetwork>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Initialize a new account.
    let account = Account::<CurrentNetwork>::new(&mut rng);
    let address = account.address();

    // Initialize a new transaction.
    let (transaction, _) = Transaction::<CurrentNetwork>::new_coinbase(address, AleoAmount(0), true, &mut rng)
        .expect("Failed to create a coinbase transaction");

    // Send the transaction to the server.
    let params = rpc_params![hex::encode(transaction.to_bytes_le().unwrap())];
    let _: <CurrentNetwork as Network>::TransactionID = rpc_client.request("sendtransaction", params).await.expect("Invalid response");

    // Fetch the transaction from the memory_pool.
    let response: Vec<Transaction<CurrentNetwork>> = rpc_client.request("getmemorypool", None).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, vec![transaction]);
}
