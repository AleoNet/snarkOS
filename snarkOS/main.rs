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

use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::{runtime::Runtime, sync::Mutex};

/// Builds a node from configuration parameters.
/// 1. Creates consensus parameters.
/// 2. Creates new storage database or uses existing.
/// 2. Creates new memory pool or uses existing from storage.
/// 3. Creates network server.
/// 4. Starts rpc server thread.
/// 5. Starts miner thread.
/// 6. Starts network server listener.
async fn start_server(config: Config) -> Result<(), NodeError> {
    if !config.node.quiet {
        std::env::set_var("RUST_LOG", "info");
        env_logger::init();

        println!("{}", render_init(&config));
    }

    let address = format! {"{}:{}", config.node.ip, config.node.port};
    let socket_address = address.parse::<SocketAddr>()?;

    let network = Network::from_network_id(config.aleo.network_id);

    let consensus = ConsensusParameters {
        max_block_size: 1_000_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
        network,
        verifier: PoswMarlin::verify_only().expect("could not instantiate PoSW verifier"),
    };

    let mut path = config.node.dir;
    path.push(&config.node.db);

    let storage = Arc::new(MerkleTreeLedger::open_at_path(path)?);

    let memory_pool = MemoryPool::from_storage(&storage.clone())?;
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool.clone()));

    let bootnode = match config.p2p.bootnodes.len() {
        0 => socket_address,
        _ => config.p2p.bootnodes[0].parse::<SocketAddr>()?,
    };

    let sync_handler = SyncHandler::new(bootnode);
    let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

    info!("Loading Aleo parameters...");
    let parameters = PublicParameters::<Components>::load(!config.miner.is_miner)?;
    info!("Loading complete.");

    let server = Server::new(
        Context::new(
            socket_address,
            config.p2p.mempool_interval,
            config.p2p.min_peers,
            config.p2p.max_peers,
            config.node.is_bootnode,
            config.p2p.bootnodes.clone(),
        ),
        consensus.clone(),
        storage.clone(),
        parameters.clone(),
        memory_pool_lock.clone(),
        sync_handler_lock.clone(),
        10000, // 10 seconds
    );

    // Start RPC thread

    if config.rpc.json_rpc {
        info!("Loading Aleo parameters for RPC...");
        let proving_parameters = PublicParameters::<Components>::load(!config.miner.is_miner)?;
        info!("Loading complete.");

        start_rpc_server(
            config.rpc.port,
            storage.clone(),
            proving_parameters,
            server.context.clone(),
            consensus.clone(),
            memory_pool_lock.clone(),
            config.rpc.username,
            config.rpc.password,
        )
        .await?;
    }

    // Start miner thread

    if config.miner.is_miner {
        match AccountAddress::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
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
            Err(_) => info!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/snarkOS.toml file or by using the --miner-address option in the CLI."
            ),
        }
    }

    // Start server thread

    server.listen().await?;

    Ok(())
}

fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::new();

    let config: Config = ConfigCli::parse(&arguments)?;

    Runtime::new()?.block_on(start_server(config))?;

    Ok(())
}
