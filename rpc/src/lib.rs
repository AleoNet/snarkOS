// Compilation
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
#![cfg_attr(nightly, warn(missing_docs))]
#![cfg_attr(nightly, doc(include = "../documentation/getting_started/00_overview.md"))]

pub mod rpc_impl;
#[doc(inline)]
pub use rpc_impl::*;

pub mod rpc_impl_protected;
#[doc(inline)]
pub use rpc_impl_protected::*;

pub mod rpc_server;
#[doc(inline)]
pub use rpc_server::*;

pub mod rpc_trait;
#[doc(inline)]
pub use rpc_trait::*;

pub mod rpc_types;
#[doc(inline)]
pub use rpc_types::*;
