use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use jsonrpc_core::{types::error::Error, BoxFuture, IoHandler, Result};
use jsonrpc_http_server::{
    cors,
    hyper::{server::Server, service::make_service_fn, Body, Request},
    RestApi,
    Rpc,
    ServerHandler,
};
use tokio::sync::{oneshot, RwLock};

use snarkos_storage::Metadata;
use snarkvm::{
    dpc::{AleoAmount, Block, BlockHeader, Blocks, MemoryPool, Network, Record, Transaction, Transactions, Transition},
    utilities::{FromBytes, ToBytes},
};

use super::NodeRPC;
use crate::{helpers::Status, Environment, LedgerReader, Peers, ProverRequest, ProverRouter};

#[inline]
fn from_internal_display_error<E: std::fmt::Display>(e: E) -> Error {
    from_internal_string(format!("{}", e))
}

#[inline]
fn from_internal_string(s: String) -> Error {
    let mut err = Error::internal_error();
    err.message = s;
    err
}

pub async fn initialize_rpc_server<N: Network, E: Environment>(
    rpc_addr: SocketAddr,
    _username: String,
    _password: String,
    status: &Status,
    peers: &Arc<Peers<N, E>>,
    ledger: LedgerReader<N>,
    prover_router: ProverRouter<N>,
    memory_pool: Arc<RwLock<MemoryPool<N>>>,
) -> tokio::task::JoinHandle<()> {
    let node_impl = NodeRPCImpl::new(rpc_addr, status.clone(), peers.clone(), ledger, prover_router, memory_pool);
    let mut io_hdl = IoHandler::new();
    io_hdl.extend_with(node_impl.to_delegate());

    let r = Rpc {
        handler: Arc::new(io_hdl.into()),
        extractor: Arc::new(|_: &Request<Body>| ()),
    };

    let service_fn = make_service_fn(move |_addr_stream| {
        let srv_hdl = ServerHandler::new(
            r.downgrade(),
            None,
            None,
            cors::AccessControlAllowHeaders::Any,
            None,
            Arc::new(|req: Request<Body>| req.into()),
            RestApi::Disabled,
            None,
            5 * 1024 * 1024,
            true,
        );
        async { Ok::<_, Infallible>(srv_hdl) }
    });

    let server = Server::bind(&rpc_addr).serve(service_fn);

    let (router, handler) = oneshot::channel();
    let task = tokio::spawn(async move {
        // Notify the outer function that the task is ready.
        let _ = router.send(());
        server.await.expect("Failed to start the RPC server");
    });
    // Wait until the spawned task is ready.
    let _ = handler.await;

    task
}

pub struct NodeRPCImpl<N: Network, E: Environment> {
    local_addr: SocketAddr,
    status: Status,
    peers: Arc<Peers<N, E>>,
    ledger: LedgerReader<N>,
    prover_router: ProverRouter<N>,
    memory_pool: Arc<RwLock<MemoryPool<N>>>,
}

impl<N: Network, E: Environment> NodeRPCImpl<N, E> {
    pub fn new(
        local_addr: SocketAddr,
        status: Status,
        peers: Arc<Peers<N, E>>,
        ledger: LedgerReader<N>,
        prover_router: ProverRouter<N>,
        memory_pool: Arc<RwLock<MemoryPool<N>>>,
    ) -> Self {
        Self {
            local_addr,
            status,
            peers,
            ledger,
            prover_router,
            memory_pool,
        }
    }
}

impl<N: Network, E: Environment> NodeRPC<N> for NodeRPCImpl<N, E> {
    fn latest_block(&self) -> Result<Block<N>> {
        Ok(self.ledger.latest_block())
    }

    fn latest_block_height(&self) -> Result<u32> {
        Ok(self.ledger.latest_block_height())
    }

    fn latest_cumulative_weight(&self) -> Result<u128> {
        Ok(self.ledger.latest_cumulative_weight())
    }

    fn latest_block_hash(&self) -> Result<N::BlockHash> {
        Ok(self.ledger.latest_block_hash())
    }

    fn latest_block_header(&self) -> Result<BlockHeader<N>> {
        Ok(self.ledger.latest_block_header())
    }

    fn latest_block_transactions(&self) -> Result<Transactions<N>> {
        Ok(self.ledger.latest_block_transactions())
    }

    fn latest_ledger_root(&self) -> Result<N::LedgerRoot> {
        Ok(self.ledger.latest_ledger_root())
    }

    fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.ledger.get_block(block_height).map_err(from_internal_display_error)
    }

    fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        let safe_start_height = std::cmp::max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        self.ledger
            .get_blocks(safe_start_height, end_block_height)
            .map_err(from_internal_display_error)
    }

    fn get_block_height(&self, block_hash: N::BlockHash) -> Result<u32> {
        self.ledger.get_block_height(&block_hash).map_err(from_internal_display_error)
    }

    fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.ledger.get_block_hash(block_height).map_err(from_internal_display_error)
    }

    fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        let safe_start_height = std::cmp::max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        self.ledger
            .get_block_hashes(safe_start_height, end_block_height)
            .map_err(from_internal_display_error)
    }

    fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.ledger.get_block_header(block_height).map_err(from_internal_display_error)
    }

    fn get_block_template(&self) -> BoxFuture<Result<serde_json::Value>> {
        let mem_pool = self.memory_pool.clone();
        let ledger = self.ledger.clone();

        Box::pin(async move {
            // Fetch the latest state from the ledger.
            let latest_block = ledger.latest_block();
            let ledger_root = ledger.latest_ledger_root();

            // Prepare the new block.
            let previous_block_hash = latest_block.hash();
            let block_height = ledger.latest_block_height() + 1;
            let block_timestamp = chrono::Utc::now().timestamp();

            // Compute the block difficulty target and cumulative_weight.
            let previous_timestamp = latest_block.timestamp();
            let previous_difficulty_target = latest_block.difficulty_target();
            let difficulty_target = Blocks::<N>::compute_difficulty_target(previous_timestamp, previous_difficulty_target, block_timestamp);
            let cumulative_weight = latest_block
                .cumulative_weight()
                .saturating_add((u64::MAX / difficulty_target) as u128);

            // Compute the coinbase reward (not including the transaction fees).
            let mut coinbase_reward = Block::<N>::block_reward(block_height);
            let mut transaction_fees = AleoAmount::ZERO;

            let transactions: Vec<String> = mem_pool
                .read()
                .await
                .transactions()
                .iter()
                .filter(|transaction| {
                    for serial_number in transaction.serial_numbers() {
                        if let Ok(true) = ledger.contains_serial_number(serial_number) {
                            return false;
                        }
                    }

                    for commitment in transaction.commitments() {
                        if let Ok(true) = ledger.contains_commitment(commitment) {
                            return false;
                        }
                    }

                    transaction_fees = transaction_fees.add(transaction.value_balance());
                    true
                })
                .map(|tx| tx.to_string())
                .collect();

            // Calculate the final coinbase reward (including the transaction fees).
            coinbase_reward = coinbase_reward.add(transaction_fees);

            Ok(serde_json::json!({
                "previous_block_hash": previous_block_hash,
                "block_height": block_height,
                "time": block_timestamp,
                "difficulty_target": difficulty_target,
                "cumulative_weight": cumulative_weight,
                "ledger_root": ledger_root,
                "transactions": transactions,
                "coinbase_reward": coinbase_reward,
            }))
        })
    }

    fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        self.ledger
            .get_block_transactions(block_height)
            .map_err(from_internal_display_error)
    }

    fn get_ciphertext(&self, commitment: N::Commitment) -> Result<N::RecordCiphertext> {
        self.ledger.get_ciphertext(&commitment).map_err(from_internal_display_error)
    }

    fn get_ledger_proof(&self, record_commitment: N::Commitment) -> Result<String> {
        let ledger_proof = self
            .ledger
            .get_ledger_inclusion_proof(record_commitment)
            .map_err(from_internal_display_error)?;

        let binary = ledger_proof
            .to_bytes_le()
            .map_err(|e| from_internal_string(format!("serialize to bytes: {}", e)))?;

        Ok(hex::encode(binary))
    }

    fn get_memory_pool(&self) -> BoxFuture<Result<Vec<Transaction<N>>>> {
        let mem_pool = self.memory_pool.clone();
        Box::pin(async move { Ok(mem_pool.read().await.transactions()) })
    }

    fn get_transaction(&self, transaction_id: N::TransactionID) -> Result<serde_json::Value> {
        let transaction: Transaction<N> = self.ledger.get_transaction(&transaction_id).map_err(from_internal_display_error)?;

        let metadata: Metadata<N> = self
            .ledger
            .get_transaction_metadata(&transaction_id)
            .map_err(from_internal_display_error)?;

        let decrypted_records: Vec<Record<N>> = transaction.to_records().collect();

        Ok(serde_json::json!({ "transaction": transaction, "metadata": metadata, "decrypted_records": decrypted_records }))
    }

    fn get_transition(&self, transition_id: N::TransitionID) -> Result<Transition<N>> {
        self.ledger.get_transition(&transition_id).map_err(from_internal_display_error)
    }

    fn get_connected_peers(&self) -> BoxFuture<Result<Vec<SocketAddr>>> {
        let peers = self.peers.clone();
        Box::pin(async move { Ok(peers.connected_peers().await) })
    }

    fn get_node_state(&self) -> BoxFuture<Result<serde_json::Value>> {
        let latest_block_hash = self.ledger.latest_block_hash();
        let latest_block_height = self.ledger.latest_block_height();
        let latest_cumulative_weight = self.ledger.latest_cumulative_weight();
        let status = self.status.to_string();
        let peers = self.peers.clone();

        Box::pin(async move {
            let candidate_peers = peers.candidate_peers().await;
            let connected_peers = peers.connected_peers().await;
            let number_of_candidate_peers = candidate_peers.len();
            let number_of_connected_peers = connected_peers.len();
            let number_of_connected_sync_nodes = peers.number_of_connected_sync_nodes().await;

            Ok(serde_json::json!({
                "candidate_peers": candidate_peers,
                "connected_peers": connected_peers,
                "latest_block_hash": latest_block_hash,
                "latest_block_height": latest_block_height,
                "latest_cumulative_weight": latest_cumulative_weight,
                "number_of_candidate_peers": number_of_candidate_peers,
                "number_of_connected_peers": number_of_connected_peers,
                "number_of_connected_sync_nodes": number_of_connected_sync_nodes,
                "software": format!("snarkOS {}", env!("CARGO_PKG_VERSION")),
                "status": status,
                "type": E::NODE_TYPE,
                "version": E::MESSAGE_VERSION,
            }))
        })
    }

    fn send_transaction(&self, transaction_bytes: String) -> BoxFuture<Result<N::TransactionID>> {
        let prover_router = self.prover_router.clone();
        let local_addr = self.local_addr;
        Box::pin(async move {
            let data = hex::decode(transaction_bytes).map_err(|e| from_internal_string(format!("hex docode: {}", e)))?;
            let transaction: Transaction<N> =
                FromBytes::from_bytes_le(&data).map_err(|e| from_internal_string(format!("deserialize from bytes: {}", e)))?;

            let transaction_id = transaction.transaction_id();

            let request = ProverRequest::UnconfirmedTransaction(local_addr, transaction);
            if let Err(error) = prover_router.send(request).await {
                warn!("[UnconfirmedTransaction] {}", error);
            }

            Ok(transaction_id)
        })
    }
}
