use crate::io::Result as IoResult;

use std::path::PathBuf;

pub trait Storage: Sized {
    /// Stores `self` to the `path`
    fn store(&self, path: &PathBuf) -> IoResult<()>;

    /// Stores `self` from the `path`
    fn load(_path: &PathBuf) -> IoResult<Self>;
}
