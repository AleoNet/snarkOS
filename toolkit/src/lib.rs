#[macro_use]
extern crate thiserror;

pub mod account;
pub mod errors;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
