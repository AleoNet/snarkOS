use crate::{RpcFunctions, RpcImpl};
use snarkos_consensus::{miner::MemoryPool, ConsensusParameters, GM17Verifier};
use snarkos_dpc::base_dpc::instantiated::{MerkleTreeLedger, Tx};
use snarkos_network::context::Context;

use jsonrpc_http_server::ServerBuilder;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

/// Starts a local JSON-RPC HTTP server at rpc_port in a new thread.
/// Rpc failures will error on the thread level but not affect the main network server.
/// This may be changed in the future to give the node more control of the rpc server.
pub async fn start_rpc_server(
    rpc_port: u16,
    storage: Arc<MerkleTreeLedger>,
    server_context: Arc<Context>,
    consensus: ConsensusParameters<GM17Verifier>,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc_server: SocketAddr = format!("127.0.0.1:{}", rpc_port).parse()?;

    let rpc_impl = RpcImpl::new(storage, server_context, consensus, memory_pool_lock);
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(rpc_impl.to_delegate());

    let server = ServerBuilder::new(io).threads(1).start_http(&rpc_server)?;

    tokio::task::spawn(async move {
        server.wait();
    });

    Ok(())
}
