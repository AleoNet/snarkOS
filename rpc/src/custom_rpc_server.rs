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
    rpc_trait::RpcFunctions,
    rpc_types::{Meta, RpcCredentials},
    RpcImpl,
};
use snarkos_consensus::MerkleTreeLedger;
use snarkos_network::Node;
use snarkvm_objects::Storage;

use hyper::{
    body::HttpBody,
    service::{make_service_fn, service_fn},
    Body,
    Server,
};
use jsonrpc_core::Params;
use tokio::task;

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

const INTERNAL_ERROR: isize = -32603;
const METHOD_NOT_FOUND: isize = -32601;
const PARSE_ERROR: isize = -32700;
const SERVER_ERROR: isize = -32000;

const METHODS_EXPECTING_PARAMS: [&str; 14] = [
    // public
    "getblock",
    "getblockhash",
    "getrawtransaction",
    "gettransactioninfo",
    "decoderawtransaction",
    "sendtransaction",
    "validaterawtransaction",
    // private
    "createrawtransaction",
    "createtransactionkernel",
    "createtransaction",
    "getrawrecord",
    "decoderecord",
    "decryptrecord",
    "disconnect",
];

#[allow(clippy::too_many_arguments)]
pub fn start_rpc_server<S: Storage + Send + Sync + 'static>(
    rpc_addr: SocketAddr,
    secondary_storage: Arc<MerkleTreeLedger<S>>,
    node_server: Node<S>,
    username: Option<String>,
    password: Option<String>,
) -> task::JoinHandle<()> {
    let credentials = match (username, password) {
        (Some(username), Some(password)) => Some(RpcCredentials { username, password }),
        _ => None,
    };

    let rpc_impl = RpcImpl::new(secondary_storage, credentials, node_server);

    let service = make_service_fn(move |_conn| {
        let rpc = rpc_impl.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_rpc(rpc.clone(), req))) }
    });

    let server = Server::bind(&rpc_addr).serve(service);

    task::spawn(async move {
        server.await.expect("The RPC server couldn't be started!");
    })
}

async fn handle_rpc<S: Storage + Send + Sync + 'static>(
    rpc: RpcImpl<S>,
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
        _ => {
            let err = json_rpc2::Error::InvalidRequest {
                data: "Invalid request!".into(),
            };
            let resp: json_rpc2::Response = err.into();

            return Ok(hyper::Response::new(
                serde_json::to_vec(&resp).unwrap_or_default().into(),
            ));
        }
    };

    // Deserialize the JSON-RPC request.
    let mut req = match json_rpc2::from_slice(&data) {
        Ok(req) => req,
        Err(_) => {
            let err = json_rpc2::Error::InvalidRequest {
                data: "Invalid request!".into(),
            };
            let resp: json_rpc2::Response = err.into();

            return Ok(hyper::Response::new(
                serde_json::to_vec(&resp).unwrap_or_default().into(),
            ));
        }
    };

    // Read the request params.
    let mut params = match read_params(&mut req) {
        Ok(params) => params,
        Err(e) => {
            let resp: json_rpc2::Response = (&mut req, e).into();
            let body = serde_json::to_vec(&resp).unwrap_or_default();

            return Ok(hyper::Response::new(body.into()));
        }
    };

    // Handle the request method.
    let value = match req.method() {
        // public
        "getblock" => {
            let resp = rpc
                .get_block(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getblockcount" => {
            let resp = rpc.get_block_count().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getbestblockhash" => {
            let resp = rpc.get_best_block_hash().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getblockhash" => match serde_json::from_value::<u32>(params.remove(0)) {
            Ok(height) => {
                let resp = rpc.get_block_hash(height).map_err(convert_crate_err);
                serde_json::to_value(resp).unwrap_or_default()
            }
            Err(_) => {
                let err = json_rpc2::RpcError {
                    code: PARSE_ERROR, // as per JSON-RPC 2.0 spec
                    message: "The request can't be parsed: invalid block height!".into(),
                    data: None,
                };
                serde_json::to_value(&err).unwrap_or_default()
            }
        },
        "getrawtransaction" => {
            let resp = rpc
                .get_raw_transaction(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "gettransactioninfo" => {
            let resp = rpc
                .get_transaction_info(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "decoderawtransaction" => {
            let resp = rpc
                .decode_raw_transaction(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "sendtransaction" => {
            let resp = rpc
                .send_raw_transaction(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "validaterawtransaction" => {
            let resp = rpc
                .validate_raw_transaction(params[0].as_str().unwrap_or("").into())
                .map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getconnectioncount" => {
            let resp = rpc.get_connection_count().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getpeerinfo" => {
            let resp = rpc.get_peer_info().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getnodeinfo" => {
            let resp = rpc.get_node_info().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getnodestats" => {
            let resp = rpc.get_node_stats().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getblocktemplate" => {
            let resp = rpc.get_block_template().map_err(convert_crate_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        // private
        "createaccount" => {
            let resp = rpc
                .create_account_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "createrawtransaction" => {
            let resp = rpc
                .create_raw_transaction_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "createtransactionkernel" => {
            let resp = rpc
                .create_transaction_kernel_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "createtransaction" => {
            let resp = rpc
                .create_transaction_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getrecordcommitments" => {
            let resp = rpc
                .get_record_commitments_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "getrawrecord" => {
            let resp = rpc
                .get_raw_record_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "decoderecord" => {
            let resp = rpc
                .decode_record_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "decryptrecord" => {
            let resp = rpc
                .decrypt_record_protected(Params::Array(params), meta)
                .await
                .map_err(convert_core_err);
            serde_json::to_value(resp).unwrap_or_default()
        }
        "disconnect" => {
            let resp = rpc.disconnect_protected(Params::Array(params), meta).await;
            serde_json::to_value(resp).unwrap_or_default()
        }
        unknown => {
            let err = json_rpc2::RpcError {
                code: METHOD_NOT_FOUND, // as per JSON-RPC 2.0 spec
                message: format!("Unknown method!: \"{}\"", unknown),
                data: None,
            };
            serde_json::to_value(&err).unwrap_or_default()
        }
    };

    // Create and serialize the response object.
    let resp = json_rpc2::Response::from((&mut req, value));
    let body = serde_json::to_vec(&resp).unwrap_or_default();

    // Send the HTTP response.
    Ok(hyper::Response::new(body.into()))
}

/// Ensures that the params are a non-empty (this assumption is taken advantage of later) array and returns them.
fn read_params(req: &mut json_rpc2::Request) -> json_rpc2::Result<Vec<serde_json::Value>> {
    if METHODS_EXPECTING_PARAMS.contains(&req.method()) {
        match req.deserialize() {
            Ok(Params::Array(arr)) if !arr.is_empty() => Ok(arr),
            Ok(_) => Err(json_rpc2::Error::InvalidParams {
                id: req.id().clone(),
                data: "Only a non-empty array is accepted as parameters!".into(),
            }),
            Err(e) => Err(e),
        }
    } else {
        Ok(vec![]) // unused in methods other than METHODS_EXPECTING_PARAMS
    }
}

/// Converts the crate's RpcError into a json_rpc2::RpcError
fn convert_crate_err(err: crate::error::RpcError) -> json_rpc2::RpcError {
    json_rpc2::RpcError {
        code: SERVER_ERROR, // as per JSON-RPC 2.0 spec
        message: err.to_string(),
        data: None,
    }
}

/// Converts the jsonrpc-core's Error into a json_rpc2::RpcError
fn convert_core_err(err: jsonrpc_core::Error) -> json_rpc2::RpcError {
    json_rpc2::RpcError {
        code: INTERNAL_ERROR, // as per JSON-RPC 2.0 spec
        message: err.to_string(),
        data: None,
    }
}
