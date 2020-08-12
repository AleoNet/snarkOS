#[macro_use]
extern crate log;

use snarkos::{
    cli::CLI,
    config::{Config, ConfigCli},
    display::render_init,
};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::PublicParameters};
use snarkos_errors::node::NodeError;
use snarkos_network::{
    context::Context,
    protocol::SyncHandler,
    server::{MinerInstance, Server},
};
use snarkos_objects::{AccountAddress, Network};
use snarkos_posw::PoswMarlin;
use snarkos_rpc::start_rpc_server;

use dirs::home_dir;
use std::{fs, net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

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

        println!("{}", render_init(&config.miner_address));
    }

    let address = format! {"{}:{}", config.ip, config.port};
    let socket_address = address.parse::<SocketAddr>()?;

    let network_id = Network::from_network_id(config.network);

    let consensus = ConsensusParameters {
        max_block_size: 1_000_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
        network: network_id,
        verifier: PoswMarlin::verify_only().expect("could not instantiate PoSW verifier"),
    };

    let mut path = home_dir().unwrap_or(std::env::current_dir()?);
    path.push(".snarkOS/");
    fs::create_dir_all(&path).map_err(|err| NodeError::Message(err.to_string()))?;
    path.push(&config.path);

    let storage = Arc::new(MerkleTreeLedger::open_at_path(path)?);

    let memory_pool = MemoryPool::from_storage(&storage.clone())?;
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool.clone()));

    let bootnode = match config.bootnodes.len() {
        0 => socket_address,
        _ => config.bootnodes[0].parse::<SocketAddr>()?,
    };

    let sync_handler = SyncHandler::new(bootnode);
    let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

    info!("Loading Aleo parameters...");
    let parameters = PublicParameters::<Components>::load(!config.is_miner)?;
    info!("Loading complete.");

    let server = Server::new(
        Context::new(
            socket_address,
            config.mempool_interval,
            config.min_peers,
            config.max_peers,
            config.is_bootnode,
            config.bootnodes.clone(),
        ),
        consensus.clone(),
        storage.clone(),
        parameters.clone(),
        memory_pool_lock.clone(),
        sync_handler_lock.clone(),
        10000, // 10 seconds
    );

    // Start rpc thread

    if config.jsonrpc {
        info!("Loading Aleo parameters for RPC...");
        let proving_parameters = PublicParameters::<Components>::load(false)?;
        info!("Loading complete.");

        start_rpc_server(
            config.rpc_port,
            storage.clone(),
            proving_parameters,
            server.context.clone(),
            consensus.clone(),
            memory_pool_lock.clone(),
            config.rpc_username,
            config.rpc_password,
        )
        .await?;
    }

    // Start miner thread

    if config.is_miner {
        let miner_address = AccountAddress::<Components>::from_str(&config.miner_address)?;
        MinerInstance::new(
            miner_address,
            consensus.clone(),
            parameters,
            storage.clone(),
            memory_pool_lock.clone(),
            server.context.clone(),
        )
        .spawn();
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
