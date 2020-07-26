// Compilation
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![deny(missing_docs)]
#![feature(external_doc)]
#![doc(include = "../documentation/overview.md")]

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
