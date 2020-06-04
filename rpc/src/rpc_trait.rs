use crate::rpc_types::*;
use snarkos_errors::rpc::RpcError;

use jsonrpc_derive::rpc;

#[rpc]
pub trait RpcFunctions {
    /// Adds two numbers and returns a result
    #[rpc(name = "add")]
    fn add(&self, a: u64, b: u64) -> Result<u64, RpcError>;

    #[rpc(name = "getblock")]
    fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError>;

    #[rpc(name = "getblockcount")]
    fn get_block_count(&self) -> Result<u32, RpcError>;

    #[rpc(name = "getbestblockhash")]
    fn get_best_block_hash(&self) -> Result<String, RpcError>;

    #[rpc(name = "getblockhash")]
    fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError>;

    #[rpc(name = "getrawtransaction")]
    fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError>;

    #[rpc(name = "decoderawtransaction")]
    fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError>;

    #[rpc(name = "sendtransaction")]
    fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError>;

    //    #[rpc(name = "decoderecord")]
    //    fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError>;

    #[rpc(name = "getconnectioncount")]
    fn get_connection_count(&self) -> Result<usize, RpcError>;

    #[rpc(name = "getpeerinfo")]
    fn get_peer_info(&self) -> Result<PeerInfo, RpcError>;

    #[rpc(name = "getblocktemplate")]
    fn get_block_template(&self) -> Result<BlockTemplate, RpcError>;
}
