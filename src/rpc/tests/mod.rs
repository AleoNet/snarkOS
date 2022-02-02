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

use crate::{
    environment::Client,
    helpers::State,
    ledger::Ledger,
    network::{Operator, Prover},
    rpc::*,
    Environment,
    Peers,
};
use snarkos_storage::{
    storage::{rocksdb::RocksDB, Storage},
    LedgerState,
};
use snarkvm::{
    dpc::{testnet2::Testnet2, AccountScheme, Address, AleoAmount, Network, Transaction, Transactions, Transition},
    prelude::{Account, Block, BlockHeader},
    utilities::ToBytes,
};

use jsonrpsee::{
    core::{client::ClientT, Error as JsonrpseeError},
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
    types::error::METHOD_NOT_FOUND_CODE,
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

/// Returns the HTTP address corresponding to a socket address.
fn http_addr(addr: SocketAddr) -> String {
    format!("http://{}", addr)
}

/// Initializes a new instance of the ledger state.
fn new_ledger_state<N: Network, S: Storage, P: AsRef<Path>>(path: Option<P>) -> LedgerState<N> {
    match path {
        Some(path) => LedgerState::<N>::open_writer::<S, _>(path).expect("Failed to initialize ledger"),
        None => LedgerState::<N>::open_writer::<S, _>(temp_dir()).expect("Failed to initialize ledger"),
    }
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

    E::tasks().append(rpc_server_handle);

    rpc_server_addr
}

fn new_rpc_client(rpc_server_addr: SocketAddr) -> HttpClient {
    HttpClientBuilder::default()
        .build(http_addr(rpc_server_addr))
        .expect("Couldn't build a JSON-RPC client")
}

#[tokio::test]
async fn test_handle_rpc() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Perform a new request with an empty body.
    let response: Result<serde_json::Value, _> = rpc_client.request("", None).await;

    // Expect an error response.
    if let Err(JsonrpseeError::Request(error)) = response {
        // Verify the error code.
        let json: serde_json::Value = serde_json::from_str(&error).expect("The response is not valid JSON");
        assert!(json["error"]["code"] == METHOD_NOT_FOUND_CODE);
    } else {
        panic!("Should have received an error response");
    }
}

#[tokio::test]
async fn test_latest_block() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: Block<Testnet2> = rpc_client.request("latestblock", None).await.expect("Invalid response");

    // Check the block.
    assert_eq!(response, *Testnet2::genesis_block());
}

#[tokio::test]
async fn test_latest_block_height() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: u32 = rpc_client.request("latestblockheight", None).await.expect("Invalid response");

    // Check the block height.
    assert_eq!(response, Testnet2::genesis_block().height());
}

#[tokio::test]
async fn test_latest_block_hash() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: <Testnet2 as Network>::BlockHash = rpc_client.request("latestblockhash", None).await.expect("Invalid response");

    // Check the block hash.
    assert_eq!(response, Testnet2::genesis_block().hash());
}

#[tokio::test]
async fn test_latest_block_header() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: BlockHeader<Testnet2> = rpc_client.request("latestblockheader", None).await.expect("Invalid response");

    // Check the block header.
    assert_eq!(response, *Testnet2::genesis_block().header());
}

#[tokio::test]
async fn test_latest_block_transactions() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: Transactions<Testnet2> = rpc_client.request("latestblocktransactions", None).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, *Testnet2::genesis_block().transactions());
}

#[tokio::test]
async fn test_latest_ledger_root() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(temp_dir()).await;
    let rpc_server_addr = new_rpc_server::<_, _, RocksDB>(Some(rpc_server_context.clone())).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: <Testnet2 as Network>::LedgerRoot = rpc_client.request("latestledgerroot", None).await.expect("Invalid response");

    // Obtain the expected result directly.
    let expected = rpc_server_context.latest_ledger_root().await.unwrap();

    // Check the ledger root.
    assert_eq!(response, expected);
}

#[tokio::test]
async fn test_get_block() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: Block<Testnet2> = rpc_client.request("getblock", params).await.expect("Invalid response");

    // Check the block.
    assert_eq!(response, *Testnet2::genesis_block());
}

#[tokio::test]
async fn test_get_blocks() {
    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize an empty ledger.
    let ledger_state = LedgerState::open_writer::<RocksDB, _>(directory.clone()).expect("Failed to initialize ledger");

    // Read the test blocks; note: they don't include the genesis block, as it's always available when creating a ledger.
    // note: the `blocks_100` file was generated on a testnet2 storage using `LedgerState::dump_blocks`.
    let test_blocks = fs::read("storage/benches/blocks_1").unwrap_or_else(|_| panic!("Missing the test blocks file"));
    let blocks: Vec<Block<Testnet2>> = bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump");

    // Load a test block into the ledger.
    ledger_state.add_next_block(&blocks[0]).expect("Failed to add a test block");

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32, 1];
    let response: Vec<Block<Testnet2>> = rpc_client.request("getblocks", params).await.expect("Invalid response");

    // Check the blocks.
    assert_eq!(response, vec![Testnet2::genesis_block().clone(), blocks[0].clone()]);
}

#[tokio::test]
async fn test_get_block_height() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![Testnet2::genesis_block().hash().to_string()];
    let response: u32 = rpc_client.request("getblockheight", params).await.expect("Invalid response");

    // Check the block height.
    assert_eq!(response, Testnet2::genesis_block().height());
}

#[tokio::test]
async fn test_get_block_hash() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: <Testnet2 as Network>::BlockHash = rpc_client.request("getblockhash", params).await.expect("Invalid response");

    // Check the block hash.
    assert_eq!(response, Testnet2::genesis_block().hash());
}

#[tokio::test]
async fn test_get_block_hashes() {
    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize an empty ledger.
    let ledger_state = LedgerState::open_writer::<RocksDB, _>(directory.clone()).expect("Failed to initialize ledger");

    // Read the test blocks; note: they don't include the genesis block, as it's always available when creating a ledger.
    // note: the `blocks_100` file was generated on a testnet2 storage using `LedgerState::dump_blocks`.
    let test_blocks = fs::read("storage/benches/blocks_1").unwrap_or_else(|_| panic!("Missing the test blocks file"));
    let blocks: Vec<Block<Testnet2>> = bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump");

    // Load a test block into the ledger.
    ledger_state.add_next_block(&blocks[0]).expect("Failed to add a test block");

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(Some(rpc_server_context)).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32, 1];
    let response: Vec<<Testnet2 as Network>::BlockHash> = rpc_client.request("getblockhashes", params).await.expect("Invalid response");

    // Check the block hashes.
    assert_eq!(response, vec![Testnet2::genesis_block().hash(), blocks[0].hash()]);
}

#[tokio::test]
async fn test_get_block_header() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: BlockHeader<Testnet2> = rpc_client.request("getblockheader", params).await.expect("Invalid response");

    // Check the block header.
    assert_eq!(response, *Testnet2::genesis_block().header());
}

#[tokio::test]
async fn test_get_block_template() {
    // Initialize an RPC context.
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(temp_dir()).await;

    // Initialize the expected block template values.
    let expected_previous_block_hash = Testnet2::genesis_block().hash().to_string();
    let expected_block_height = 1;
    let expected_ledger_root = rpc_server_context.latest_ledger_root().await.unwrap().to_string();
    let expected_transactions = Vec::<serde_json::Value>::new();
    let expected_block_reward = Block::<Testnet2>::block_reward(1).0;

    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(Some(rpc_server_context)).await;
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
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![0u32];
    let response: Transactions<Testnet2> = rpc_client.request("getblocktransactions", params).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, *Testnet2::genesis_block().transactions());
}

#[tokio::test]
async fn test_get_ciphertext() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Get the commitment from the genesis coinbase transaction.
    let commitment = Testnet2::genesis_block().to_coinbase_transaction().unwrap().transitions()[0]
        .commitments()
        .next()
        .unwrap()
        .to_string();

    // Send the request to the server.
    let params = rpc_params![commitment];
    let response: <Testnet2 as Network>::RecordCiphertext = rpc_client.request("getciphertext", params).await.expect("Invalid response");

    // Check the ciphertext.
    assert!(
        Testnet2::genesis_block()
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
    let ledger_state = new_ledger_state::<Testnet2, RocksDB, PathBuf>(Some(directory.clone()));
    assert_eq!(0, ledger_state.latest_block_height());

    // Initialize a new account.
    let account = Account::<Testnet2>::new(&mut rng);
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
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(Some(rpc_server_context)).await;
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
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let response: serde_json::Value = rpc_client.request("getnodestate", None).await.expect("Invalid response");

    // Declare the expected node state.
    let expected = serde_json::json!({
        "address": Option::<Address<Testnet2>>::None,
        "candidate_peers": Vec::<SocketAddr>::new(),
        "connected_peers": Vec::<SocketAddr>::new(),
        "latest_block_hash": Testnet2::genesis_block().hash(),
        "latest_block_height": 0u32,
        "latest_cumulative_weight": "0",
        "launched": format!("{} minutes ago", 0),
        "number_of_candidate_peers": 0usize,
        "number_of_connected_peers": 0usize,
        "number_of_connected_sync_nodes": 0usize,
        "software": format!("snarkOS {}", env!("CARGO_PKG_VERSION")),
        "status": Client::<Testnet2>::status().to_string(),
        "type": Client::<Testnet2>::NODE_TYPE,
        "version": Client::<Testnet2>::MESSAGE_VERSION,
    });

    // Check the node state.
    assert_eq!(response, expected);
}

#[tokio::test]
async fn test_get_transaction() {
    /// Additional metadata included with a transaction response
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct GetTransactionResponse {
        pub transaction: Transaction<Testnet2>,
        pub metadata: snarkos_storage::Metadata<Testnet2>,
        pub decrypted_records: Vec<Record<Testnet2>>,
    }

    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize a new ledger state at the temporary directory.
    let ledger_state = new_ledger_state::<Testnet2, RocksDB, PathBuf>(Some(directory.clone()));

    // Prepare the expected values.
    let transaction_id = Testnet2::genesis_block().to_coinbase_transaction().unwrap().transaction_id();
    let expected_transaction_metadata = ledger_state.get_transaction_metadata(&transaction_id).unwrap();
    let expected_transaction = Testnet2::genesis_block().transactions().first().unwrap();
    let expected_decrypted_records: Vec<Record<Testnet2>> = expected_transaction.to_records().collect();

    // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
    drop(ledger_state);

    // Initialize a new RPC server and create an associated client.
    let rpc_server_context = new_rpc_context::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(directory).await;
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(Some(rpc_server_context)).await;
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
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Get a transition ID from the genesis coinbase transaction.
    let transition_id = Testnet2::genesis_block().to_coinbase_transaction().unwrap().transitions()[0]
        .transition_id()
        .to_string();

    // Send the request to the server.
    let params = rpc_params![transition_id];
    let response: Transition<Testnet2> = rpc_client.request("gettransition", params).await.expect("Invalid response");

    // Check the transition.
    assert!(
        Testnet2::genesis_block()
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
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
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
    let account = Account::<Testnet2>::new(&mut rng);
    let address = account.address();

    // Initialize a new transaction.
    let (transaction, _) =
        Transaction::<Testnet2>::new_coinbase(address, AleoAmount(1234), true, &mut rng).expect("Failed to create a coinbase transaction");

    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![hex::encode(transaction.to_bytes_le().unwrap())];
    let response: <Testnet2 as Network>::TransactionID = rpc_client.request("sendtransaction", params).await.expect("Invalid response");

    // Check the transaction id.
    assert_eq!(response, transaction.transaction_id());
}

#[tokio::test]
async fn test_send_transaction_large() {
    // Initialize a new RPC server and create an associated client.
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Send the request to the server.
    let params = rpc_params![include_str!("large_test_tx")];
    let response: <Testnet2 as Network>::TransactionID = rpc_client.request("sendtransaction", params).await.expect("Invalid response");

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
    let rpc_server_addr = new_rpc_server::<Testnet2, Client<Testnet2>, RocksDB>(None).await;
    let rpc_client = new_rpc_client(rpc_server_addr);

    // Initialize a new account.
    let account = Account::<Testnet2>::new(&mut rng);
    let address = account.address();

    // Initialize a new transaction.
    let (transaction, _) =
        Transaction::<Testnet2>::new_coinbase(address, AleoAmount(0), true, &mut rng).expect("Failed to create a coinbase transaction");

    // Send the transaction to the server.
    let params = rpc_params![hex::encode(transaction.to_bytes_le().unwrap())];
    let _: <Testnet2 as Network>::TransactionID = rpc_client.request("sendtransaction", params).await.expect("Invalid response");

    // Fetch the transaction from the memory_pool.
    let response: Vec<Transaction<Testnet2>> = rpc_client.request("getmemorypool", None).await.expect("Invalid response");

    // Check the transactions.
    assert_eq!(response, vec![transaction]);
}
