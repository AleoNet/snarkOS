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

//! Logic for instantiating the RPC server.

use crate::{
    helpers::Status,
    rpc::{rpc_impl::RpcImpl, rpc_trait::RpcFunctions},
    Environment,
    LedgerReader,
    Peers,
    ProverRouter,
};
use snarkvm::dpc::Network;

use hyper::{
    body::HttpBody,
    server::{conn::AddrStream, Server},
    service::{make_service_fn, service_fn},
    Body,
};
use json_rpc_types as jrt;
use jsonrpc_core::{Metadata, Params};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;

/// Defines the authentication format for accessing private endpoints on the RPC server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RpcCredentials {
    /// The username in the credential
    pub username: String,
    /// The password in the credential
    pub password: String,
}

/// RPC metadata for encoding authentication.
#[derive(Default, Clone)]
pub struct Meta {
    /// An optional authentication string for protected RPC functions.
    pub auth: Option<String>,
}

impl Metadata for Meta {}

const METHODS_EXPECTING_PARAMS: [&str; 12] = [
    // public
    "getblock",
    "getblocks",
    "getblockheight",
    "getblockhash",
    "getblockhashes",
    "getblockheader",
    "getblocktransactions",
    "getciphertext",
    "getledgerproof",
    "gettransaction",
    "gettransition",
    "sendtransaction",
    // "validaterawtransaction",
    // // private
    // "createrawtransaction",
    // "createtransaction",
    // "getrawrecord",
    // "decoderecord",
    // "decryptrecord",
    // "disconnect",
    // "connect",
];

/// Starts a local RPC HTTP server at `rpc_port` in a dedicated `tokio` task.
/// RPC failures do not affect the rest of the node.
pub async fn initialize_rpc_server<N: Network, E: Environment>(
    rpc_addr: SocketAddr,
    username: String,
    password: String,
    status: &Status,
    peers: &Arc<Peers<N, E>>,
    ledger: LedgerReader<N>,
    prover_router: ProverRouter<N>,
) -> tokio::task::JoinHandle<()> {
    let credentials = RpcCredentials { username, password };
    let rpc_impl = RpcImpl::new(credentials, status.clone(), peers.clone(), ledger, prover_router);

    let service = make_service_fn(move |conn: &AddrStream| {
        let caller = conn.remote_addr();
        let rpc = rpc_impl.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_rpc::<N, E>(caller, rpc.clone(), req))) }
    });

    let server = Server::bind(&rpc_addr).serve(service);

    let (router, handler) = oneshot::channel();
    let task = tokio::spawn(async move {
        // Notify the outer function that the task is ready.
        let _ = router.send(());
        server.await.expect("Failed to start the RPC server");
    });
    // Wait until the spawned task is ready.
    let _ = handler.await;

    task
}

async fn handle_rpc<N: Network, E: Environment>(
    caller: SocketAddr,
    rpc: RpcImpl<N, E>,
    req: hyper::Request<Body>,
) -> Result<hyper::Response<Body>, Infallible> {
    // Obtain the username and password, if present.
    let auth = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .map(|h| h.to_str().unwrap_or("").to_owned());
    let _meta = Meta { auth };

    // Save the headers.
    let headers = req.headers().clone();

    // Ready the body of the request
    let mut body = req.into_body();
    let data = match body.data().await {
        Some(Ok(data)) => data,
        err_or_none => {
            let mut error = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Couldn't read the RPC body");
            if let Some(Err(err)) = err_or_none {
                error.data = Some(err.to_string());
            }

            let resp = jrt::Response::<(), String>::error(jrt::Version::V2, error, None);
            let body = serde_json::to_vec(&resp).unwrap_or_default();

            return Ok(hyper::Response::new(body.into()));
        }
    };

    // Deserialize the JSON-RPC request.
    let req: jrt::Request<Params> = match serde_json::from_slice(&data) {
        Ok(req) => req,
        Err(_) => {
            let resp = jrt::Response::<(), ()>::error(
                jrt::Version::V2,
                jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Couldn't parse the RPC body"),
                None,
            );
            let body = serde_json::to_vec(&resp).unwrap_or_default();

            return Ok(hyper::Response::new(body.into()));
        }
    };

    debug!("Received '{}' RPC request from {}: {:?}", &*req.method, caller, headers);

    // Read the request params.
    let mut params = match read_params(&req) {
        Ok(params) => params,
        Err(err) => {
            let resp = jrt::Response::<(), ()>::error(jrt::Version::V2, err, req.id.clone());
            let body = serde_json::to_vec(&resp).unwrap_or_default();

            return Ok(hyper::Response::new(body.into()));
        }
    };

    // Handle the request method.
    let response = match &*req.method {
        // Public
        "latestblock" => {
            let result = rpc.latest_block().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "latestblockheight" => {
            let result = rpc.latest_block_height().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "latestblockhash" => {
            let result = rpc.latest_block_hash().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "latestblockheader" => {
            let result = rpc.latest_block_header().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "latestblocktransactions" => {
            let result = rpc.latest_block_transactions().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "latestledgerroot" => {
            let result = rpc.latest_ledger_root().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "getblock" => match serde_json::from_value::<u32>(params.remove(0)) {
            Ok(height) => {
                let result = rpc.get_block(height).await.map_err(convert_crate_err);
                result_to_response(&req, result)
            }
            Err(_) => {
                let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                jrt::Response::error(jrt::Version::V2, err, req.id.clone())
            }
        },
        "getblocks" => {
            match (
                serde_json::from_value::<u32>(params.remove(0)),
                serde_json::from_value::<u32>(params.remove(0)),
            ) {
                (Ok(start_block_height), Ok(end_block_height)) => {
                    let result = rpc
                        .get_blocks(start_block_height, end_block_height)
                        .await
                        .map_err(convert_crate_err);
                    result_to_response(&req, result)
                }
                (Err(_), _) | (_, Err(_)) => {
                    let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                    jrt::Response::error(jrt::Version::V2, err, req.id.clone())
                }
            }
        }
        "getblockheight" => {
            let result = rpc.get_block_height(params.remove(0)).await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "getblockhash" => match serde_json::from_value::<u32>(params.remove(0)) {
            Ok(height) => {
                let result = rpc.get_block_hash(height).await.map_err(convert_crate_err);
                result_to_response(&req, result)
            }
            Err(_) => {
                let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                jrt::Response::error(jrt::Version::V2, err, req.id.clone())
            }
        },
        "getblockhashes" => {
            match (
                serde_json::from_value::<u32>(params.remove(0)),
                serde_json::from_value::<u32>(params.remove(0)),
            ) {
                (Ok(start_block_height), Ok(end_block_height)) => {
                    let result = rpc
                        .get_block_hashes(start_block_height, end_block_height)
                        .await
                        .map_err(convert_crate_err);
                    result_to_response(&req, result)
                }
                (Err(_), _) | (_, Err(_)) => {
                    let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                    jrt::Response::error(jrt::Version::V2, err, req.id.clone())
                }
            }
        }
        "getblockheader" => match serde_json::from_value::<u32>(params.remove(0)) {
            Ok(height) => {
                let result = rpc.get_block_header(height).await.map_err(convert_crate_err);
                result_to_response(&req, result)
            }
            Err(_) => {
                let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                jrt::Response::error(jrt::Version::V2, err, req.id.clone())
            }
        },
        "getblocktransactions" => match serde_json::from_value::<u32>(params.remove(0)) {
            Ok(height) => {
                let result = rpc.get_block_transactions(height).await.map_err(convert_crate_err);
                result_to_response(&req, result)
            }
            Err(_) => {
                let err = jrt::Error::with_custom_msg(jrt::ErrorCode::ParseError, "Invalid block height!");
                jrt::Response::error(jrt::Version::V2, err, req.id.clone())
            }
        },
        "getciphertext" => {
            let result = rpc.get_ciphertext(params.remove(0)).await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "getledgerproof" => {
            let result = rpc.get_ledger_proof(params.remove(0)).await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "gettransaction" => {
            let result = rpc.get_transaction(params.remove(0)).await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "gettransition" => {
            let result = rpc.get_transition(params.remove(0)).await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "getconnectedpeers" => {
            let result = rpc.get_connected_peers().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "getnodestate" => {
            let result = rpc.get_node_state().await.map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        "sendtransaction" => {
            let result = rpc
                .send_transaction(params[0].as_str().unwrap_or("").into())
                .await
                .map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        // "getblocktemplate" => {
        //     let result = rpc.get_block_template().await.map_err(convert_crate_err);
        //     result_to_response(&req, result)
        // }
        // // private
        // "createaccount" => {
        //     let result = rpc
        //         .create_account_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "createrawtransaction" => {
        //     let result = rpc
        //         .create_raw_transaction_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "createtransaction" => {
        //     let result = rpc
        //         .create_transaction_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "getrecordcommitments" => {
        //     let result = rpc
        //         .get_record_commitments_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "getrawrecord" => {
        //     let result = rpc
        //         .get_raw_record_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "decryptrecord" => {
        //     let result = rpc
        //         .decrypt_record_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        // "connect" => {
        //     let result = rpc
        //         .connect_protected(Params::Array(params), meta)
        //         .await
        //         .map_err(convert_core_err);
        //     result_to_response(&req, result)
        // }
        _ => {
            let err = jrt::Error::from_code(jrt::ErrorCode::MethodNotFound);
            jrt::Response::error(jrt::Version::V2, err, req.id.clone())
        }
    };

    // Serialize the response object.
    let body = serde_json::to_vec(&response).unwrap_or_default();

    // Send the HTTP response.
    Ok(hyper::Response::new(body.into()))
}

/// Ensures that the params are a non-empty (this assumption is taken advantage of later) array and returns them.
fn read_params(req: &jrt::Request<Params>) -> Result<Vec<serde_json::Value>, jrt::Error<()>> {
    if METHODS_EXPECTING_PARAMS.contains(&&*req.method) {
        match &req.params {
            Some(Params::Array(arr)) if !arr.is_empty() => Ok(arr.clone()),
            Some(_) => Err(jrt::Error::from_code(jrt::ErrorCode::InvalidParams)),
            None => Err(jrt::Error::from_code(jrt::ErrorCode::InvalidParams)),
        }
    } else {
        Ok(vec![]) // unused in methods other than METHODS_EXPECTING_PARAMS
    }
}

/// Converts the crate's RpcError into a jrt::RpcError
fn convert_crate_err(err: crate::rpc::rpc_impl::RpcError) -> jrt::Error<String> {
    let error = jrt::Error::with_custom_msg(jrt::ErrorCode::ServerError(-32000), "internal error");
    error.set_data(err.to_string())
}

/// Converts the jsonrpc-core's Error into a jrt::RpcError
#[allow(unused)]
fn convert_core_err(err: jsonrpc_core::Error) -> jrt::Error<String> {
    let error = jrt::Error::with_custom_msg(jrt::ErrorCode::InternalError, "JSONRPC server error");
    error.set_data(err.to_string())
}

fn result_to_response<T: Serialize>(
    request: &jrt::Request<Params>,
    result: Result<T, jrt::Error<String>>,
) -> jrt::Response<serde_json::Value, String> {
    match result {
        Ok(res) => {
            let result = serde_json::to_value(&res).unwrap_or_default();
            jrt::Response::result(jrt::Version::V2, result, request.id.clone())
        }
        Err(err) => jrt::Response::error(jrt::Version::V2, err, request.id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    use crate::helpers::Tasks;
    use snarkos_ledger::{
        storage::{rocksdb::RocksDB, Storage},
        LedgerState,
    };
    use snarkvm::{
        dpc::{testnet2::Testnet2, AccountScheme, AleoAmount, RecordCiphertext, Transaction, Transactions, Transition},
        prelude::{Account, Block, BlockHeader},
        utilities::ToBytes,
    };

    use hyper::Request;
    use rand::{thread_rng, SeedableRng};
    use rand_chacha::ChaChaRng;
    use std::{
        path::{Path, PathBuf},
        str::FromStr,
        sync::atomic::AtomicBool,
    };
    use tokio::sync::mpsc;

    fn temp_dir() -> std::path::PathBuf {
        tempfile::tempdir().expect("Failed to open temporary directory").into_path()
    }

    /// Returns a dummy caller IP address.
    fn caller() -> SocketAddr {
        "0.0.0.0:3030".to_string().parse().unwrap()
    }

    /// Initializes a new instance of the `Peers` struct.
    async fn peers<N: Network, E: Environment>() -> Arc<Peers<N, E>> {
        Peers::new(&mut Tasks::new(), "0.0.0.0:4130".parse().unwrap(), None, &Status::new()).await
    }

    /// Initializes a new instance of the ledger state.
    fn new_ledger_state<N: Network, S: Storage, P: AsRef<Path>>(path: Option<P>) -> LedgerState<N> {
        match path {
            Some(path) => LedgerState::<N>::open_writer::<S, _>(path).expect("Failed to initialize ledger"),
            None => LedgerState::<N>::open_writer::<S, _>(temp_dir()).expect("Failed to initialize ledger"),
        }
    }

    /// Initializes a new instance of the rpc.
    async fn new_rpc<N: Network, E: Environment, S: Storage, P: AsRef<Path>>(path: Option<P>) -> RpcImpl<N, E> {
        let credentials = RpcCredentials {
            username: "root".to_string(),
            password: "pass".to_string(),
        };
        let ledger = Arc::new(new_ledger_state::<N, S, P>(path));

        // Create a dummy mpsc channel for Prover requests. todo (@collinc97): only get requests will work until this is changed
        let (prover_router, _prover_handler) = mpsc::channel(1024);

        RpcImpl::<N, E>::new(credentials, Status::new(), peers::<N, E>().await, ledger, prover_router)
    }

    /// Deserializes a rpc response into the given type.
    async fn process_response<T: serde::de::DeserializeOwned>(response: hyper::Response<Body>) -> T {
        assert!(response.status().is_success());

        let response_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let response_json: jrt::Response<serde_json::Value, String> = serde_json::from_slice(&response_bytes).unwrap();

        serde_json::from_value(response_json.payload.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn test_handle_rpc() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request with an empty body.
        let request = Request::new(Body::empty());

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request).await;

        // Check the response was received without errors.
        assert!(response.is_ok());
        assert!(response.unwrap().status().is_success());
    }

    #[tokio::test]
    async fn test_latest_block() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `latestblock` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestblock"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block.
        let actual: Block<Testnet2> = process_response(response).await;

        // Check the block.
        let expected = Testnet2::genesis_block();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_latest_block_height() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `latestblockheight` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestblockheight"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block height.
        let actual: u32 = process_response(response).await;

        // Check the block height.
        let expected = Testnet2::genesis_block().height();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_latest_block_hash() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `latestblockhash` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestblockhash"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block hash.
        let actual: <Testnet2 as Network>::BlockHash = process_response(response).await;

        // Check the block hash.
        let expected = Testnet2::genesis_block().hash();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_latest_block_header() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `latestblockheader` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestblockheader"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block header.
        let actual: BlockHeader<Testnet2> = process_response(response).await;

        // Check the block header.
        let expected = Testnet2::genesis_block().header();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_latest_block_transactions() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `latestblocktransactions` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestblocktransactions"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into transactions.
        let actual: Transactions<Testnet2> = process_response(response).await;

        // Check the transactions.
        let expected = Testnet2::genesis_block().transactions();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_latest_ledger_root() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        let expected = rpc.latest_ledger_root().await.unwrap();

        // Initialize a new request that calls the `latestledgerroot` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc":"2.0",
	"id": "1",
	"method": "latestledgerroot"
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a ledger root.
        let actual: <Testnet2 as Network>::LedgerRoot = process_response(response).await;

        // Check the ledger root.
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_get_block() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getblock` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getblock",
	"params": [
        0
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block.
        let actual: Block<Testnet2> = process_response(response).await;

        // Check the block.
        let expected = Testnet2::genesis_block();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_get_blocks() {
        let rng = &mut thread_rng();
        let terminator = AtomicBool::new(false);

        // Initialize a new temporary directory.
        let directory = temp_dir();

        // Initialize a new ledger state at the temporary directory.
        let ledger_state = new_ledger_state::<Testnet2, RocksDB, PathBuf>(Some(directory.clone()));
        assert_eq!(0, ledger_state.latest_block_height());

        // Initialize a new account.
        let account = Account::<Testnet2>::new(&mut thread_rng());
        let address = account.address();

        // Mine the next block.
        let block_1 = ledger_state
            .mine_next_block(address, &[], &terminator, rng)
            .expect("Failed to mine");
        ledger_state.add_next_block(&block_1).expect("Failed to add next block to ledger");
        assert_eq!(1, ledger_state.latest_block_height());

        // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
        drop(ledger_state);

        // Initialize a new rpc with the ledger state containing the genesis block and block_1.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(Some(directory.clone())).await;

        // Initialize a new request that calls the `getblocks` endpoint.
        let request = Request::new(Body::from(
            r#"{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "getblocks",
    "params": [
        0, 1
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into blocks.
        let actual: Vec<Block<Testnet2>> = process_response(response).await;

        // Check the blocks.
        let expected = vec![Testnet2::genesis_block(), &block_1];
        expected.into_iter().zip(actual.into_iter()).for_each(|(expected, actual)| {
            assert_eq!(*expected, actual);
        });
    }

    #[tokio::test]
    async fn test_get_block_height() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getblockheight` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getblockheight",
	"params": [
        "ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block height.
        let actual: u32 = process_response(response).await;

        // Check the block height.
        let expected = Testnet2::genesis_block().height();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_get_block_hash() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getblockhash` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getblockhash",
	"params": [
        0
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block hash.
        let actual: <Testnet2 as Network>::BlockHash = process_response(response).await;

        // Check the block hash.
        let expected = Testnet2::genesis_block().hash();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_get_block_hashes() {
        let rng = &mut thread_rng();
        let terminator = AtomicBool::new(false);

        // Initialize a new temporary directory.
        let directory = temp_dir();

        // Initialize a new ledger state at the temporary directory.
        let ledger_state = new_ledger_state::<Testnet2, RocksDB, PathBuf>(Some(directory.clone()));
        assert_eq!(0, ledger_state.latest_block_height());

        // Initialize a new account.
        let account = Account::<Testnet2>::new(&mut thread_rng());
        let address = account.address();

        // Mine the next block.
        let block_1 = ledger_state
            .mine_next_block(address, &[], &terminator, rng)
            .expect("Failed to mine");
        ledger_state.add_next_block(&block_1).expect("Failed to add next block to ledger");
        assert_eq!(1, ledger_state.latest_block_height());

        // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
        drop(ledger_state);

        // Initialize a new rpc with the ledger state containing the genesis block and block_1.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(Some(directory.clone())).await;

        // Initialize a new request that calls the `getblockhashes` endpoint.
        let request = Request::new(Body::from(
            r#"{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "getblockhashes",
    "params": [
        0, 1
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into block hashes.
        let actual: Vec<<Testnet2 as Network>::BlockHash> = process_response(response).await;

        // Check the block hashes.
        let expected = vec![Testnet2::genesis_block().hash(), block_1.hash()];
        expected.into_iter().zip(actual.into_iter()).for_each(|(expected, actual)| {
            assert_eq!(expected, actual);
        });
    }

    #[tokio::test]
    async fn test_get_block_header() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getblockheader` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getblockheader",
	"params": [
        0
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a block header.
        let actual: BlockHeader<Testnet2> = process_response(response).await;

        // Check the block header.
        let expected = Testnet2::genesis_block().header();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_get_block_transactions() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getblocktransactions` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getblocktransactions",
	"params": [
        0
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into transactions.
        let actual: Transactions<Testnet2> = process_response(response).await;

        // Check the transactions.
        let expected = Testnet2::genesis_block().transactions();
        assert_eq!(*expected, actual);
    }

    #[tokio::test]
    async fn test_get_ciphertext() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `getciphertext` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getciphertext",
	"params": [
        "ar18gr9hxzr40ve9238eddus8vq7ka8a07wk63666hmdqk7ess5mqqsh5xazm"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a ciphertext.
        let actual: RecordCiphertext<Testnet2> = process_response(response).await;

        // Check the ciphertext.
        assert!(
            Testnet2::genesis_block()
                .transactions()
                .first()
                .unwrap()
                .ciphertexts()
                .any(|expected| *expected == actual)
        );
    }

    #[tokio::test]
    async fn test_get_ledger_proof() {
        let mut rng = ChaChaRng::seed_from_u64(123456789);
        let terminator = AtomicBool::new(false);

        // Initialize a new temporary directory.
        let directory = temp_dir();

        // Initialize a new ledger state at the temporary directory.
        let ledger_state = new_ledger_state::<Testnet2, RocksDB, PathBuf>(Some(directory.clone()));
        assert_eq!(0, ledger_state.latest_block_height());

        // Initialize a new account.
        let account = Account::<Testnet2>::new(&mut rng);
        let address = account.address();

        // Mine the next block.
        let block_1 = ledger_state
            .mine_next_block(address, &[], &terminator, &mut rng)
            .expect("Failed to mine");
        ledger_state.add_next_block(&block_1).expect("Failed to add next block to ledger");
        assert_eq!(1, ledger_state.latest_block_height());

        // Get the record commitment.
        let decrypted_records = block_1.transactions().first().unwrap().to_decrypted_records(account.view_key());
        assert!(!decrypted_records.is_empty());
        let record_commitment = decrypted_records[0].commitment();

        // Get the ledger proof.
        let ledger_proof = ledger_state.get_ledger_inclusion_proof(record_commitment).unwrap();

        // Drop the handle to ledger_state. Note this does not remove the blocks in the temporary directory.
        drop(ledger_state);

        // Initialize a new rpc with the ledger state containing the genesis block and block_1.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(Some(directory.clone())).await;

        // Initialize a new request that calls the `getledgerproof` endpoint.
        let request = Request::new(Body::from(
            r#"{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "getledgerproof",
    "params": [
        "cm10pzlc5xkvuj9hpd8lnp3mzsl3e7g622fxfh7skxh74l7ycmcs5rqrlcrw5"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a ledger proof string.
        let actual: String = process_response(response).await;

        // Check the ledger proof.
        let expected = hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof"));
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn test_get_transaction() {
        /// Additional metadata included with a transaction response
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub struct GetTransactionResponse {
            pub transaction: Transaction<Testnet2>,
            pub metadata: snarkos_ledger::Metadata<Testnet2>,
        }

        // Initialize a new ledger.
        let ledger = new_ledger_state::<Testnet2, RocksDB, PathBuf>(None);

        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `gettransaction` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "gettransaction",
	"params": [
        "at1pazplqjlhvyvex64xrykr4egpt77z05n74u5vlnkyv05r3ctgyxs0cgj6w"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a transaction and transaction metadata.
        let actual: GetTransactionResponse = process_response(response).await;

        // Check the transaction.
        assert_eq!(*Testnet2::genesis_block().transactions().first().unwrap(), actual.transaction);

        // Check the metadata.
        let expected_transaction_metadata = ledger
            .get_transaction_metadata(
                &<Testnet2 as Network>::TransactionID::from_str("at1pazplqjlhvyvex64xrykr4egpt77z05n74u5vlnkyv05r3ctgyxs0cgj6w").unwrap(),
            )
            .unwrap();

        assert_eq!(expected_transaction_metadata, actual.metadata);
    }

    #[tokio::test]
    async fn test_get_transition() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `gettransition` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "gettransition",
	"params": [
        "as15d8a5nrc86xn5cqmfd208wmn3xa9ul3y9l7w8eys4gj6637awvqskxa3ef"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a transition.
        let actual: Transition<Testnet2> = process_response(response).await;

        // Check the transition.
        assert!(
            Testnet2::genesis_block()
                .transactions()
                .first()
                .unwrap()
                .transitions()
                .iter()
                .any(|expected| *expected == actual)
        );
    }

    #[tokio::test]
    async fn test_get_connected_peers() {
        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `gettransition` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "getconnectedpeers",
	"params": []
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a transition.
        let actual: Vec<String> = process_response(response).await;

        // Check the transition.
        assert_eq!(actual, Vec::<String>::new());
    }

    #[tokio::test]
    async fn test_send_transaction() {
        let mut rng = ChaChaRng::seed_from_u64(123456789);

        // Initialize a new account.
        let account = Account::<Testnet2>::new(&mut rng);
        let address = account.address();

        // Initialize a new transaction.
        let transaction =
            Transaction::<Testnet2>::new_coinbase(address, AleoAmount(1234), &mut rng).expect("Failed to create a coinbase transaction");

        // Initialize a new rpc.
        let rpc = new_rpc::<Testnet2, Client<Testnet2>, RocksDB, PathBuf>(None).await;

        // Initialize a new request that calls the `sendtransaction` endpoint.
        let request = Request::new(Body::from(
            r#"{
	"jsonrpc": "2.0",
	"id": "1",
	"method": "sendtransaction",
	"params": [
        "a8a2358b0aa49123434b1757172e034d119cc282cfceb42210f886920ae6754d043539a82da297d5578504fcab003300ccecab3ae59f05892805b378bcdbacfa4fd24818e4048c8a09e98e034cb6790c0100ccb7bfe9e0933daa1b446a4cab28717459db173754d9535ba0bffc5b8c304c0981407a85e3220300896eab8c06ec5fa91701c553c0d7734b9d357fb8a6dea1006e9862d7eed6f07666f41c734f4f8ed80031b45cb12bf54be8d3c025ffe51f120674438b929cfcefb97470058145ebecc711c101102c4d8d388cbbd15c2759072b70ebde4aa0ae864ed39302e39d56a12a7e58416204545e1300741965120a0e985d7c48ad6b925f8f88f9c0bd33a8563e780a0786e1f898bfe65addb2e3f80b5c10ece2389438fcea239bd737e412e29227cdefdc74a37d80132a03b0f5060c124d0b46b99747a7aeeceff1589c08c6eae67cd2de4eb3810e245e654b5f1e05126ec4432ff09c76776f1a8169f1e72959f1eedd0cc48d4349587ab0025aa208fed92eda33f6e4f06048df167ddabae4e651cfc1cd7339b02cf5d11ef76df009d8770d20502290ee7fc3aa005882992419dafa16f9002b0ec872296212cdb20f2a6f2593346c2129188b14446ab2e45522f1b977e38a305bfbaa4199097cc103f90756979113bcde9aeb84d9930cfe1d67f7116f41b878ba9b501ceb7daa400a9de48e187395dc67a674b02bd6a455f56eb08bee168f96579be85d090844c004d697e8ec02e0ca759ec904a8a3991c015b65523ee24a4aea89b2eda0da773d11aac359834fe3647ee20c08b10bccd39b638aed2d24d6fb2268739e390ed9850b9db40835eb280e4a43c8fd444c408fa9f038f5a23ac42b30846269d507fdd90e0309e3144dd23baef11d1c3a806cc81de2ce55dd31e8595d753be30f84a0a509d005e9b057d2fa2ddf559b4ad8f50cd86fc3ebb928c0ab8a6eb96806171e85026a619d98561decb838a85760b971cad2ce7496796454a0b8b11d80f7f2139a03cd5d670d229fe3cdee33abfde96bd4e60fddfed184a88b446a3dd20b5b0dbb0a644aac3ca4c3590b18b63ae0190f83316faba89db05b7bf339bf14aa478f410572e86a32c8f7b22c7e479cbbea00934243d2b45339d2ffbd68db439e23d04c0e93b694efe478ebf8816fa259c8f85f873eb4f51ed7a037c6577a2f5f7907e3082efbffffffffffff95ff564b8f87c7541b871536baa809a1f5902f37c23a98a00f73bd4d48c567a12eda954376db7cb4d6889957224e0946985bc13106c2b2359523786e1d443a33b4362ef3e51ec2f550ef15d428809c9a3985da1a8dd8a95d1b808538be0a2200f744755a098ecf7106edfb3795396c6e477eeb5b4f435299edfa549502c91b79c83b623039731d23b41878e74fd5680b186f6601898fe109cfe6e2816252a2684a225d3d7c93060bd32b91ddb4c3e3512c87c61d4fa624fc8cddaa0a353f1181c096a058d420b23c4b435c1857356e4c8eed7605df993bac56b9a860cdf9b7e6492116e77c380d7c1d15c853a4298ffa84facfc18010eac98db7391376d86214d9a1474c89c3d259c1d69da8e455718d433f5cbf62de324705771b55358fbf00010000"
    ]
}"#,
        ));

        // Send the request to the rpc.
        let response = handle_rpc(caller(), rpc, request)
            .await
            .expect("Test rpc failed to process request");

        // Process the response into a ciphertext.
        let actual: <Testnet2 as Network>::TransactionID = process_response(response).await;

        // Check the transaction id.
        let expected = transaction.transaction_id();
        assert_eq!(expected, actual);
    }
}
