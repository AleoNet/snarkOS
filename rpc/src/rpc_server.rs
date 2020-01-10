use crate::{RpcFunctions, RpcImpl};
use snarkos_consensus::{miner::MemoryPool, ConsensusParameters};
use snarkos_network::base::{Context, Message};
use snarkos_storage::BlockStorage;

use jsonrpc_http_server::ServerBuilder;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, Mutex};

pub async fn start_rpc_server(
    rpc_port: u16,
    storage: Arc<BlockStorage>,
    sender: mpsc::Sender<(Message, SocketAddr)>,
    server_context: Arc<Context>,
    consensus: ConsensusParameters,
    memory_pool_lock: Arc<Mutex<MemoryPool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc_server: SocketAddr = format!("127.0.0.1:{}", rpc_port).parse()?;

    let rpc_impl = RpcImpl::new(storage, sender, rpc_server, server_context, consensus, memory_pool_lock);
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(rpc_impl.to_delegate());

    let server = ServerBuilder::new(io).threads(1).start_http(&rpc_server)?;

    tokio::task::spawn(async move {
        server.wait();
    });

    Ok(())
}
