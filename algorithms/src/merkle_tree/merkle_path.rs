use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::{MerkleParameters, CRH};
use snarkos_utilities::ToBytes;

pub type MerkleTreeDigest<P> = <<P as MerkleParameters>::H as CRH>::Output;

/// Stores the hashes of a particular path (in order) from leaf to root.
/// Our path `is_left_child()` if the boolean in `path` is true.
#[derive(Clone, Debug)]
pub struct MerklePath<P: MerkleParameters> {
    pub parameters: P,
    pub path: Vec<(<P::H as CRH>::Output, <P::H as CRH>::Output)>,
}

impl<P: MerkleParameters> MerklePath<P> {
    pub fn verify<L: ToBytes>(&self, root_hash: &<P::H as CRH>::Output, leaf: &L) -> Result<bool, MerkleError> {
        if self.path.len() != P::DEPTH {
            return Ok(false);
        }

        // Check that the given leaf matches the leaf in the membership proof.
        if !self.path.is_empty() {
            let hash_input_size_in_bytes = (P::H::INPUT_SIZE_BITS / 8) * 2;
            let mut buffer = vec![0u8; hash_input_size_in_bytes];

            let claimed_leaf_hash = self.parameters.hash_leaf::<L>(leaf, &mut buffer)?;

            // Check if leaf is one of the bottom-most siblings.
            if claimed_leaf_hash != self.path[0].0 && claimed_leaf_hash != self.path[0].1 {
                return Ok(false);
            };

            // Check levels between leaf level and root.
            let mut previous_hash = claimed_leaf_hash;
            let mut buffer = vec![0u8; hash_input_size_in_bytes];
            for &(ref hash, ref sibling_hash) in &self.path {
                // Check if the previous hash matches the correct current hash.
                if &previous_hash != hash && &previous_hash != sibling_hash {
                    return Ok(false);
                };
                previous_hash = self.parameters.hash_inner_node(hash, sibling_hash, &mut buffer)?;
            }

            if root_hash != &previous_hash {
                return Ok(false);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<P: MerkleParameters> Default for MerklePath<P> {
    fn default() -> Self {
        let mut path = Vec::with_capacity(P::DEPTH);
        for _i in 0..P::DEPTH {
            path.push((<P::H as CRH>::Output::default(), <P::H as CRH>::Output::default()));
        }
        Self {
            parameters: P::default(),
            path,
        }
    }
}
