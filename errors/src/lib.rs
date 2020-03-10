#[macro_use]
extern crate failure;

pub mod algorithms;
pub mod consensus;
pub mod curves;
pub mod dpc;
pub mod gadgets;
pub mod network;
pub mod node;
pub mod objects;
pub mod rpc;
pub mod storage;

#[macro_export]
macro_rules! unwrap_option_or_continue {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! unwrap_result_or_continue {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(_) => continue,
        }
    };
}

#[macro_export]
macro_rules! unwrap_option_or_error {
    ($e:expr; $err:expr) => {
        match $e {
            Some(val) => val,
            None => return Err($err),
        }
    };
}
