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
use snarkos_network::context::Context;

use jsonrpc_http_server::{cors::AccessControlAllowHeaders, hyper, ServerBuilder};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

/// Starts a local JSON-RPC HTTP server at rpc_port in a new thread.
/// Rpc failures will error on the thread level but not affect the main network server.
/// This may be changed in the future to give the node more control of the rpc server.
pub async fn start_rpc_server(
    rpc_port: u16,
    storage: Arc<MerkleTreeLedger>,
    parameters: PublicParameters<Components>,
    server_context: Arc<Context>,
    consensus: ConsensusParameters,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc_server: SocketAddr = format!("0.0.0.0:{}", rpc_port).parse()?;

    let credentials = match (username, password) {
        (Some(username), Some(password)) => Some(RpcCredentials { username, password }),
        _ => None,
    };

    let rpc_impl = RpcImpl::new(
        storage,
        parameters,
        server_context,
        consensus,
        memory_pool_lock,
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
