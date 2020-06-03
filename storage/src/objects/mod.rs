use std::{collections::HashSet, hash::Hash};

/// The methods defined in this module require direct access to the storage module.
/// Many are verification checks on snarkos-objects that are called by snarkos-consensus components.
/// As a result, it is difficult to determine the appropriate module for them to live in.
pub mod block;
pub use self::block::*;

pub mod block_header;
pub use self::block_header::*;

pub mod block_path;
pub use self::block_path::*;

pub mod dpc_state;
pub use self::dpc_state::*;

pub mod insert_commit;
pub use self::insert_commit::*;

pub mod ledger_scheme;
pub use self::ledger_scheme::*;

pub mod records;
pub use self::records::*;

pub mod transaction;
pub use self::transaction::*;

/// Check if an iterator has duplicate elements
pub fn has_duplicates<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    !iter.into_iter().all(move |x| uniq.insert(x))
}
