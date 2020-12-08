// Copyright (C) 2019-2020 Aleo Systems Inc.
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
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_network::{Environment, SyncManager};

use jsonrpc_http_server::{cors::AccessControlAllowHeaders, hyper, ServerBuilder};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock};

/// Starts a local JSON-RPC HTTP server at rpc_port in a new thread.
/// Rpc failures will error on the thread level but not affect the main network server.
/// This may be changed in the future to give the node more control of the rpc server.
#[allow(clippy::too_many_arguments)]
pub async fn start_rpc_server(
    rpc_port: u16,
    secondary_storage: Arc<RwLock<MerkleTreeLedger>>,
    storage_path: PathBuf,
    parameters: PublicParameters<Components>,
    environment: Environment,
    consensus: ConsensusParameters,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    sync_handler_lock: Arc<Mutex<SyncManager>>,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc_server: SocketAddr = format!("0.0.0.0:{}", rpc_port).parse()?;

    let credentials = match (username, password) {
        (Some(username), Some(password)) => Some(RpcCredentials { username, password }),
        _ => None,
    };

    let rpc_impl = RpcImpl::new(
        secondary_storage,
        storage_path,
        parameters,
        environment,
        consensus,
        memory_pool_lock,
        sync_handler_lock,
        credentials,
    );
    let mut io = jsonrpc_core::MetaIoHandler::default();

    rpc_impl.add_protected(&mut io);
    io.extend_with(rpc_impl.to_delegate());

    let server = ServerBuilder::new(io)
        .cors_allow_headers(AccessControlAllowHeaders::Any)
        .meta_extractor(|req: &hyper::Request<hyper::Body>| {
            let auth = req
                .headers()
                .get(hyper::header::AUTHORIZATION)
                .map(|h| h.to_str().unwrap_or("").to_owned());

            Meta { auth }
        })
        .threads(1)
        .start_http(&rpc_server)?;

    tokio::task::spawn(async move {
        server.wait();
    });

    Ok(())
}
