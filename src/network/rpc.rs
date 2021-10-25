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

use jsonrpc_http_server::{
    jsonrpc_core::{IoHandler, Params, Value},
    ServerBuilder,
};
use std::net::SocketAddr;
use tokio::task;

// use crate::{
//     rpc_trait::RpcFunctions,
//     rpc_types::{Meta, RpcCredentials},
//     RpcImpl,
// };

// use jsonrpc_http_server::{cors::AccessControlAllowHeaders, hyper};

/// Initializes a new local JSON-RPC HTTP server.
pub fn start_rpc_server(rpc_ip: SocketAddr, username: Option<String>, password: Option<String>) -> task::JoinHandle<()> {
    // let credentials = match (username, password) {
    //     (Some(username), Some(password)) => Some(RpcCredentials { username, password }),
    //     _ => None,
    // };

    // let rpc_impl = RpcImpl::new(storage, credentials, node_server);
    // let mut io = jsonrpc_core::MetaIoHandler::default();
    //
    // rpc_impl.add_protected(&mut io);
    // io.extend_with(rpc_impl.to_delegate());
    //
    // let server = ServerBuilder::new(io)
    //     .cors_allow_headers(AccessControlAllowHeaders::Any)
    //     .meta_extractor(|req: &hyper::Request<hyper::Body>| {
    //         let auth = req
    //             .headers()
    //             .get(hyper::header::AUTHORIZATION)
    //             .map(|h| h.to_str().unwrap_or("").to_owned());
    //
    //         Meta { auth }
    //     })
    //     .threads(1)
    //     .start_http(&rpc_ip)
    //     .expect("couldn't start the RPC server!");
    //
    // tokio::task::spawn(async move {
    //     server.wait();
    // })

    let mut io = IoHandler::default();
    io.add_method("getblock", |_params: Params| async { Ok(Value::String("hello".to_owned())) });

    let server = ServerBuilder::new(io).threads(3).start_http(&rpc_ip).unwrap();

    task::spawn(async move {
        server.wait();
    })
}
