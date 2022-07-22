// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

//! Logic for instantiating the RPC server.

use crate::RpcFunctions;
use snarkos_environment::Environment;
use snarkos_network::{LedgerReader, State};
use snarkvm::dpc::{Address, Network};

use futures::TryFutureExt;
use jsonrpsee::{
    core::{
        middleware::{Headers, HttpMiddleware, MethodKind, Params},
        Error as JsonrpseeError,
    },
    http_server::{AccessControlBuilder, HttpServerBuilder, RpcModule},
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, ops::Deref, sync::Arc, time::Instant};
use tokio::sync::oneshot;

// The details on resource-limiting can be found at https://github.com/paritytech/jsonrpsee/blob/master/core/src/server/resource_limiting.rs
// note: jsonrpsee expects string literals as resource names; we'll be distinguishing
// them by the const name, so in order for the actual lookups to be faster, we can make
// the underlying strings short, as long as they are unique.
/// The resource label corresponding to the number of all active RPC calls.
const ALL_CONCURRENT_REQUESTS: &str = "0";
/// The maximum number of RPC requests that can be handled at once at any given time.
const ALL_CONCURRENT_REQUESTS_LIMIT: u16 = 10;

#[doc(hidden)]
pub struct RpcInner<N: Network, E: Environment> {
    pub(crate) address: Option<Address<N>>,
    pub(crate) state: Arc<State<N, E>>,
    /// RPC credentials for accessing guarded endpoints
    #[allow(unused)]
    pub(crate) credentials: RpcCredentials,
    pub(crate) launched: Instant,
}

/// Implements RPC HTTP endpoint functions for a node.
#[derive(Clone)]
pub struct RpcContext<N: Network, E: Environment>(Arc<RpcInner<N, E>>);

impl<N: Network, E: Environment> Deref for RpcContext<N, E> {
    type Target = RpcInner<N, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: Network, E: Environment> RpcContext<N, E> {
    /// Creates a new struct for calling public and private RPC endpoints.
    #[allow(clippy::too_many_arguments)]
    pub fn new(username: String, password: String, address: Option<Address<N>>, state: Arc<State<N, E>>) -> Self {
        Self(Arc::new(RpcInner {
            address,
            state,
            credentials: RpcCredentials { username, password },
            launched: Instant::now(),
        }))
    }

    pub(crate) fn ledger(&self) -> &LedgerReader<N> {
        self.state.ledger().reader()
    }
}

/// Defines the authentication format for accessing private endpoints on the RPC server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RpcCredentials {
    /// The username in the credential
    pub username: String,
    /// The password in the credential
    pub password: String,
}

/// RPC metadata for encoding authentication.
#[derive(Default, Clone)]
pub struct Meta {
    /// An optional authentication string for protected RPC functions.
    pub auth: Option<String>,
}

/// An implementation of jsonrpsee's Middleware.
#[derive(Clone)]
struct RpcMiddleware;

impl HttpMiddleware for RpcMiddleware {
    type Instant = Instant;

    fn on_request(&self, _remote_addr: SocketAddr, _headers: &Headers) -> Instant {
        Instant::now()
    }

    fn on_call(&self, method_name: &str, _params: Params<'_>, _kind: MethodKind) {
        debug!("Received a '{}' RPC request", method_name);
    }

    fn on_result(&self, method_name: &str, success: bool, started_at: Instant) {
        let result = if success { "succeeded" } else { "failed" };
        trace!("Call to '{}' {} in {:?}", method_name, result, started_at.elapsed());
    }

    fn on_response(&self, _result: &str, _started_at: Self::Instant) {}
}

/// Starts a local RPC HTTP server at `rpc_port` in a dedicated `tokio` task.
/// RPC failures do not affect the rest of the node.
pub async fn initialize_rpc_server<N: Network, E: Environment>(
    rpc_server_addr: SocketAddr,
    rpc_server_context: RpcContext<N, E>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let access_control = AccessControlBuilder::default().build(); // TODO(ljedrz): adjust to only accept the desired hosts?

    let server = HttpServerBuilder::new()
        .set_access_control(access_control)
        // Limit the number of requests handled at a time to `ALL_CONCURRENT_REQUESTS_LIMIT`; the `1` argument means that all RPC requests
        // will count towards that limit by 1, meaning they all have the same weight wrt. the resource labeled `ALL_CONCURRENT_REQUESTS`.
        .register_resource(ALL_CONCURRENT_REQUESTS, ALL_CONCURRENT_REQUESTS_LIMIT, 1)
        .expect("Invalid JSON-RPC server resource")
        .max_request_body_size(10 * 1024 * 1024) // Explicitly select the body size limit (jsonrpsee's default, 10MiB) for greater visibility.
        .set_middleware(RpcMiddleware)
        .build(rpc_server_addr).await.expect("Failed to create the RPC server");

    let server_addr = server.local_addr().expect("Can't obtain RPC server's local address");

    let module = create_rpc_module(rpc_server_context).expect("Failed to start the RPC server");

    let (router, handler) = oneshot::channel();
    let task = tokio::spawn(async move {
        // Notify the outer function that the task is ready.
        let _ = router.send(());
        let server_handle = server.start(module).expect("Failed to start the RPC server");
        server_handle.await
    });
    // Wait until the spawned task is ready.
    let _ = handler.await;

    (server_addr, task)
}

fn create_rpc_module<N: Network, E: Environment>(rpc_context: RpcContext<N, E>) -> Result<RpcModule<RpcContext<N, E>>, JsonrpseeError> {
    let mut module = RpcModule::new(rpc_context);

    // Public methods.

    module.register_async_method("latestblock", |_rpc_params, rpc_context| async move {
        rpc_context.latest_block().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestblockheight", |_rpc_params, rpc_context| async move {
        rpc_context.latest_block_height().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestcumulativeweight", |_rpc_params, rpc_context| async move {
        rpc_context.latest_cumulative_weight().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestblockhash", |_rpc_params, rpc_context| async move {
        rpc_context.latest_block_hash().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestblockheader", |_rpc_params, rpc_context| async move {
        rpc_context.latest_block_header().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestblocktransactions", |_rpc_params, rpc_context| async move {
        rpc_context.latest_block_transactions().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("latestledgerroot", |_rpc_params, rpc_context| async move {
        rpc_context.latest_ledger_root().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblock", |rpc_params, rpc_context| async move {
        let height = rpc_params.parse::<[u32; 1]>()?[0];
        rpc_context.get_block(height).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblocks", |rpc_params, rpc_context| async move {
        let [start_height, end_height]: [u32; 2] = rpc_params.parse()?;
        rpc_context
            .get_blocks(start_height, end_height)
            .map_err(JsonrpseeError::to_call_error)
            .await
    })?;

    module.register_async_method("getblockheight", |rpc_params, rpc_context| async move {
        let hash = rpc_params.parse::<[N::BlockHash; 1]>()?[0];
        rpc_context.get_block_height(hash).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblockhash", |rpc_params, rpc_context| async move {
        let height = rpc_params.parse::<[u32; 1]>()?[0];
        rpc_context.get_block_hash(height).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblockhashes", |rpc_params, rpc_context| async move {
        let [start_height, end_height]: [u32; 2] = rpc_params.parse()?;
        rpc_context
            .get_block_hashes(start_height, end_height)
            .map_err(JsonrpseeError::to_call_error)
            .await
    })?;

    module.register_async_method("getblockheader", |rpc_params, rpc_context| async move {
        let height = rpc_params.parse::<[u32; 1]>()?[0];
        rpc_context.get_block_header(height).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblocktemplate", |_rpc_params, rpc_context| async move {
        rpc_context.get_block_template().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getblocktransactions", |rpc_params, rpc_context| async move {
        let height = rpc_params.parse::<[u32; 1]>()?[0];
        rpc_context
            .get_block_transactions(height)
            .map_err(JsonrpseeError::to_call_error)
            .await
    })?;

    module.register_async_method("getciphertext", |rpc_params, rpc_context| async move {
        let commitment = rpc_params.parse::<[N::Commitment; 1]>()?[0];
        rpc_context.get_ciphertext(commitment).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getledgerproof", |rpc_params, rpc_context| async move {
        let commitment = rpc_params.parse::<[N::Commitment; 1]>()?[0];
        rpc_context
            .get_ledger_proof(commitment)
            .map_err(JsonrpseeError::to_call_error)
            .await
    })?;

    module.register_async_method("getmemorypool", |_rpc_params, rpc_context| async move {
        rpc_context.get_memory_pool().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("gettransaction", |rpc_params, rpc_context| async move {
        let id = rpc_params.parse::<[N::TransactionID; 1]>()?[0];
        rpc_context.get_transaction(id).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("gettransition", |rpc_params, rpc_context| async move {
        let id = rpc_params.parse::<[N::TransitionID; 1]>()?[0];
        rpc_context.get_transition(id).map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getconnectedpeers", |_rpc_params, rpc_context| async move {
        rpc_context.get_connected_peers().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("getnodestate", |_rpc_params, rpc_context| async move {
        rpc_context.get_node_state().map_err(JsonrpseeError::to_call_error).await
    })?;

    module.register_async_method("sendtransaction", |rpc_params, rpc_context| async move {
        let string = std::mem::take(&mut rpc_params.parse::<[String; 1]>()?[0]);
        rpc_context.send_transaction(string).map_err(JsonrpseeError::to_call_error).await
    })?;

    // Private methods.

    // "createtransaction" => {
    //     let result = rpc
    //         .create_transaction_protected(Params::Array(params), meta)
    //         .await
    //         .map_err(convert_core_err);
    //     result_to_response(&req, result)
    // }
    // "getrecordcommitments" => {
    //     let result = rpc
    //         .get_record_commitments_protected(Params::Array(params), meta)
    //         .await
    //         .map_err(convert_core_err);
    //     result_to_response(&req, result)
    // }
    // "getrawrecord" => {
    //     let result = rpc
    //         .get_raw_record_protected(Params::Array(params), meta)
    //         .await
    //         .map_err(convert_core_err);
    //     result_to_response(&req, result)
    // }
    // "decryptrecord" => {
    //     let result = rpc
    //         .decrypt_record_protected(Params::Array(params), meta)
    //         .await
    //         .map_err(convert_core_err);
    //     result_to_response(&req, result)
    // }
    // "connect" => {
    //     let result = rpc
    //         .connect_protected(Params::Array(params), meta)
    //         .await
    //         .map_err(convert_core_err);
    //     result_to_response(&req, result)
    // }

    module.register_async_method("getsharesforprover", |rpc_params, rpc_context| async move {
        let prover = rpc_params.parse::<[Address<N>; 1]>()?[0];
        rpc_context
            .get_shares_for_prover(prover)
            .map_err(JsonrpseeError::to_call_error)
            .await
    })?;

    module.register_async_method("getshares", |_rpc_params, rpc_context| async move {
        let shares = rpc_context.get_shares().await;
        Ok(shares)
    })?;

    module.register_async_method("getprovers", |_rpc_params, rpc_context| async move {
        let provers = rpc_context.get_provers().await;
        Ok(provers)
    })?;

    Ok(module)
}
