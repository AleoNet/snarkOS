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
    network::Ledger,
    rpc::{rpc_impl::RpcImpl, rpc_trait::RpcFunctions},
    Environment,
    LedgerRouter,
};
use snarkvm::dpc::Network;

use hyper::{
    body::HttpBody,
    server::Server,
    service::{make_service_fn, service_fn},
    Body,
};
use json_rpc_types as jrt;
use jsonrpc_core::{Metadata, Params};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

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
pub fn initialize_rpc_server<N: Network, E: Environment>(
    rpc_addr: SocketAddr,
    username: String,
    password: String,
    ledger: Arc<RwLock<Ledger<N, E>>>,
    ledger_router: LedgerRouter<N, E>,
) -> tokio::task::JoinHandle<()> {
    let credentials = RpcCredentials { username, password };
    let rpc_impl = RpcImpl::new(credentials, ledger, ledger_router);

    let service = make_service_fn(move |_conn| {
        let rpc = rpc_impl.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_rpc::<N, E>(rpc.clone(), req))) }
    });

    let server = Server::bind(&rpc_addr).serve(service);

    tokio::spawn(async move {
        server.await.expect("The RPC server couldn't be started!");
    })
}

async fn handle_rpc<N: Network, E: Environment>(
    rpc: RpcImpl<N, E>,
    req: hyper::Request<Body>,
) -> Result<hyper::Response<Body>, Infallible> {
    // Obtain the username and password, if present.
    let auth = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .map(|h| h.to_str().unwrap_or("").to_owned());
    let meta = Meta { auth };

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
        "sendtransaction" => {
            let result = rpc
                .send_transaction(params[0].as_str().unwrap_or("").into())
                .await
                .map_err(convert_crate_err);
            result_to_response(&req, result)
        }
        // "decoderawtransaction" => {
        //     let result = rpc
        //         .decode_raw_transaction(params[0].as_str().unwrap_or("").into())
        //         .await
        //         .map_err(convert_crate_err);
        //     result_to_response(&req, result)
        // }
        // "validaterawtransaction" => {
        //     let result = rpc
        //         .validate_raw_transaction(params[0].as_str().unwrap_or("").into())
        //         .await
        //         .map_err(convert_crate_err);
        //     result_to_response(&req, result)
        // }
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
        // "decoderecord" => {
        //     let result = rpc
        //         .decode_record_protected(Params::Array(params), meta)
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
        // "disconnect" => {
        //     let result = rpc
        //         .disconnect_protected(Params::Array(params), meta)
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
