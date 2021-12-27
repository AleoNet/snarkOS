use std::net::SocketAddr;

use anyhow::anyhow;
use jsonrpc_core::{BoxFuture, Result};
use jsonrpc_core_client::transports::http;
use jsonrpc_derive::rpc;

use snarkvm::dpc::{Block, BlockHeader, Network, Transaction, Transactions, Transition};

mod server;
pub use server::initialize_rpc_server;

#[rpc]
pub trait NodeRPC<N>
where
    N: Network,
{
    #[rpc(name = "latest_block")]
    fn latest_block(&self) -> Result<Block<N>>;

    #[rpc(name = "latest_block_height")]
    fn latest_block_height(&self) -> Result<u32>;

    #[rpc(name = "latest_cumulative_weight")]
    fn latest_cumulative_weight(&self) -> Result<u128>;

    #[rpc(name = "latest_block_hash")]
    fn latest_block_hash(&self) -> Result<N::BlockHash>;

    #[rpc(name = "latest_block_header")]
    fn latest_block_header(&self) -> Result<BlockHeader<N>>;

    #[rpc(name = "latest_block_transactions")]
    fn latest_block_transactions(&self) -> Result<Transactions<N>>;

    #[rpc(name = "latest_ledger_root")]
    fn latest_ledger_root(&self) -> Result<N::LedgerRoot>;

    #[rpc(name = "get_block")]
    fn get_block(&self, block_height: u32) -> Result<Block<N>>;

    #[rpc(name = "get_blocks")]
    fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>>;

    #[rpc(name = "get_block_height")]
    fn get_block_height(&self, block_hash: N::BlockHash) -> Result<u32>;

    #[rpc(name = "get_block_hash")]
    fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash>;

    #[rpc(name = "get_block_hashes")]
    fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>>;

    #[rpc(name = "get_block_header")]
    fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>>;

    #[rpc(name = "get_block_template")]
    fn get_block_template(&self) -> BoxFuture<Result<serde_json::Value>>;

    #[rpc(name = "get_block_transactions")]
    fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>>;

    #[rpc(name = "get_ciphertext")]
    fn get_ciphertext(&self, commitment: N::Commitment) -> Result<N::RecordCiphertext>;

    #[rpc(name = "get_ledger_proof")]
    fn get_ledger_proof(&self, record_commitment: N::Commitment) -> Result<String>;

    #[rpc(name = "get_memory_pool")]
    fn get_memory_pool(&self) -> BoxFuture<Result<Vec<Transaction<N>>>>;

    #[rpc(name = "get_transaction")]
    fn get_transaction(&self, transaction_id: N::TransactionID) -> Result<serde_json::Value>;

    #[rpc(name = "get_transition")]
    fn get_transition(&self, transition_id: N::TransitionID) -> Result<Transition<N>>;

    #[rpc(name = "get_connected_peers")]
    fn get_connected_peers(&self) -> BoxFuture<Result<Vec<SocketAddr>>>;

    #[rpc(name = "get_node_state")]
    fn get_node_state(&self) -> BoxFuture<Result<serde_json::Value>>;

    #[rpc(name = "send_transaction")]
    fn send_transaction(&self, transaction_bytes: String) -> BoxFuture<Result<N::TransactionID>>;
}

pub async fn new_client<N: Network>(host: &str) -> anyhow::Result<NodeRPCClient<N>> {
    let client = http::connect(host).await.map_err(|e| anyhow!("connect to {}: {}", host, e))?;
    Ok(client)
}
