use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    io::Result as IoResult,
};

use std::path::PathBuf;

pub trait Storage: Sized + FromBytes + ToBytes {
    /// Stores `self` to the `path`
    fn store(&self, path: &PathBuf) -> IoResult<()>;

    /// Stores `self` from the `path`
    fn load(path: &PathBuf) -> IoResult<Self>;
}
