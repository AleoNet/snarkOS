use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::CRH;
use snarkos_utilities::{bytes::ToBytes, storage::Storage};

use rand::Rng;
use std::io::Cursor;

pub trait MerkleParameters: Clone + Default + Storage {
    type H: CRH;

    const HEIGHT: usize;

    /// Setup the MerkleParameters
    fn setup<R: Rng>(rng: &mut R) -> Self;

    /// Returns the collision-resistant hash function used by the Merkle tree.
    fn crh(&self) -> &Self::H;

    /// Returns the collision-resistant hash function used by the Merkle tree.
    fn parameters(&self) -> &<<Self as MerkleParameters>::H as CRH>::Parameters;

    /// Returns the hash of a given leaf.
    fn hash_leaf<L: ToBytes>(&self, leaf: &L, buffer: &mut [u8]) -> Result<<Self::H as CRH>::Output, MerkleError> {
        let mut writer = Cursor::new(buffer);
        leaf.write(&mut writer)?;

        let buffer = writer.into_inner();
        Ok(self.crh().hash(&buffer[..(Self::H::INPUT_SIZE_BITS / 8)])?)
    }

    /// Returns the output hash, given a left and right hash value.
    fn hash_inner_node(
        &self,
        left: &<Self::H as CRH>::Output,
        right: &<Self::H as CRH>::Output,
        buffer: &mut [u8],
    ) -> Result<<Self::H as CRH>::Output, MerkleError> {
        let mut writer = Cursor::new(buffer);

        // Construct left input.
        left.write(&mut writer)?;
        // Construct right input.
        right.write(&mut writer)?;

        let buffer = writer.into_inner();
        Ok(self.crh().hash(&buffer[..(<Self::H as CRH>::INPUT_SIZE_BITS / 8)])?)
    }

    fn hash_empty(&self) -> Result<<Self::H as CRH>::Output, MerkleError> {
        let empty_buffer = vec![0u8; <Self::H as CRH>::INPUT_SIZE_BITS / 8];
        Ok(self.crh().hash(&empty_buffer)?)
    }
}
