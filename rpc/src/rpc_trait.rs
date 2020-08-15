//! Definition of the public and private RPC endpoints.

use crate::rpc_types::*;
use snarkos_errors::rpc::RpcError;

use jsonrpc_derive::rpc;

/// Definition of public RPC endpoints.
#[rpc]
pub trait RpcFunctions {
    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getblock.md"))]
    #[rpc(name = "getblock")]
    fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getblockcount.md"))]
    #[rpc(name = "getblockcount")]
    fn get_block_count(&self) -> Result<u32, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getbestblockhash.md"))]
    #[rpc(name = "getbestblockhash")]
    fn get_best_block_hash(&self) -> Result<String, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getblockhash.md"))]
    #[rpc(name = "getblockhash")]
    fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getrawtransaction.md"))]
    #[rpc(name = "getrawtransaction")]
    fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/gettransactioninfo.md"))]
    #[rpc(name = "gettransactioninfo")]
    fn get_transaction_info(&self, transaction_id: String) -> Result<TransactionInfo, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/decoderawtransaction.md"))]
    #[rpc(name = "decoderawtransaction")]
    fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/sendtransaction.md"))]
    #[rpc(name = "sendtransaction")]
    fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError>;

    #[cfg_attr(
        nightly,
        doc(include = "../documentation/public_endpoints/validaterawtransaction.md")
    )]
    #[rpc(name = "validaterawtransaction")]
    fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getconnectioncount.md"))]
    #[rpc(name = "getconnectioncount")]
    fn get_connection_count(&self) -> Result<usize, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getpeerinfo.md"))]
    #[rpc(name = "getpeerinfo")]
    fn get_peer_info(&self) -> Result<PeerInfo, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/getblocktemplate.md"))]
    #[rpc(name = "getblocktemplate")]
    fn get_block_template(&self) -> Result<BlockTemplate, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/decoderecord.md"))]
    #[rpc(name = "decoderecord")]
    fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/public_endpoints/decryptrecord.md"))]
    #[rpc(name = "decryptrecord")]
    fn decrypt_record(&self, decryption_input: DecryptRecordInput) -> Result<String, RpcError>;
}

/// Definition of private RPC endpoints that require authentication.
pub trait ProtectedRpcFunctions {
    #[cfg_attr(nightly, doc(include = "../documentation/private_endpoints/createaccount.md"))]
    fn create_account(&self) -> Result<RpcAccount, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/private_endpoints/createrawtransaction.md"))]
    fn create_raw_transaction(
        &self,
        transaction_input: TransactionInputs,
    ) -> Result<CreateRawTransactionOuput, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/private_endpoints/getrecordcommitments.md"))]
    fn get_record_commitments(&self) -> Result<Vec<String>, RpcError>;

    #[cfg_attr(
        nightly,
        doc(include = "../documentation/private_endpoints/getrecordcommitmentcount.md")
    )]
    fn get_record_commitment_count(&self) -> Result<usize, RpcError>;

    #[cfg_attr(nightly, doc(include = "../documentation/private_endpoints/getrawrecord.md"))]
    fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError>;
}
