use node::{
    cli::CLI,
    config::{Config, ConfigCli},
};
use snarkos_consensus::{miner::MemoryPool, ConsensusParameters};
use snarkos_errors::node::NodeError;
use snarkos_network::{context::Context, server::Server};
use snarkos_rpc::start_rpc_server;
use snarkos_storage::BlockStorage;

use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

/// Builds a node from configuration parameters.
/// 1. Creates consensus parameters.
/// 2. Creates new storage database or uses existing.
/// 2. Creates new memory pool or uses existing from storage.
/// 3. Creates network server.
/// 4. Starts rpc server thread.
/// 5. Starts miner thread.
/// 6. Starts network server listener.
async fn start_server(config: Config) -> Result<(), NodeError> {
    if !config.quiet {
        std::env::set_var("RUST_LOG", "info");
        env_logger::init();
    }

    let address = format! {"{}:{}", config.ip, config.port};
    let local_address = address.parse::<SocketAddr>()?;

    let consensus = ConsensusParameters {
        max_block_size: 1_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
        transaction_size: 366usize,
    };

    let mut path = std::env::current_dir()?;
    path.push(&config.path);

    let storage = BlockStorage::open_at_path(path, config.genesis)?;

    let memory_pool = MemoryPool::from_storage(&storage.clone())?;
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool.clone()));

    let server = Server::new(
        consensus.clone(),
        Arc::new(Context::new(
            local_address,
            config.network,
            1u64,
            10000, //10 seconds
            config.memory_pool_interval,
            config.min_peers,
            config.max_peers,
            config.is_bootnode,
            config.bootnodes.clone(),
        )),
        storage.clone(),
        memory_pool_lock.clone(),
    );

    // Start rpc thread

    if config.jsonrpc {
        start_rpc_server(
            config.rpc_port,
            storage.clone(),
            server.context.clone(),
            consensus.clone(),
            memory_pool_lock.clone(),
        )
        .await?;
    }

    // Start miner thread

    let coinbase_address = BitcoinAddress::<Mainnet>::from_str(&config.coinbase_address).unwrap();

    if config.miner {
        server.start_miner(coinbase_address);
    }

    // Start server thread

    server.listen().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::new();

    let config: Config = ConfigCli::parse(&arguments)?;

    start_server(config).await
}
