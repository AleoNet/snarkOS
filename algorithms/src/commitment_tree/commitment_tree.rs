use crate::commitment_tree::CommitmentMerklePath;
use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::{CommitmentScheme, CRH};
use snarkos_utilities::{to_bytes, ToBytes};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: CommitmentScheme, H: CRH"),
    PartialEq(bound = "C: CommitmentScheme, H: CRH"),
    Eq(bound = "C: CommitmentScheme, H: CRH")
)]
pub struct CommitmentMerkleTree<C: CommitmentScheme, H: CRH> {
    /// The computed root of the full Merkle tree.
    root: <H as CRH>::Output,

    /// The internal hashes of the local data Merkle tree
    inner_hashes: (<H as CRH>::Output, <H as CRH>::Output),

    /// The leaves of the local data Merkle tree
    leaves: [<C as CommitmentScheme>::Output; 4],

    /// The CRH parameters used to construct the Merkle tree
    #[derivative(PartialEq = "ignore")]
    parameters: H,
}

impl<C: CommitmentScheme, H: CRH> CommitmentMerkleTree<C, H> {
    /// Construct a new commitment Merkle tree.
    pub fn new(parameters: H, leaves: &[<C as CommitmentScheme>::Output; 4]) -> Result<Self, MerkleError> {
        let input_1 = to_bytes![leaves[0], leaves[1]]?;
        let inner_hash1 = H::hash(&parameters, &input_1)?;

        let input_2 = to_bytes![leaves[2], leaves[3]]?;
        let inner_hash2 = H::hash(&parameters, &input_2)?;

        let root = H::hash(&parameters, &to_bytes![inner_hash1, inner_hash2]?)?;

        Ok(Self {
            root,
            inner_hashes: (inner_hash1, inner_hash2),
            leaves: leaves.clone(),
            parameters,
        })
    }

    #[inline]
    pub fn root(&self) -> <H as CRH>::Output {
        self.root.clone()
    }

    #[inline]
    pub fn inner_hashes(&self) -> (<H as CRH>::Output, <H as CRH>::Output) {
        self.inner_hashes.clone()
    }

    #[inline]
    pub fn leaves(&self) -> [<C as CommitmentScheme>::Output; 4] {
        self.leaves.clone()
    }

    pub fn generate_proof(
        &self,
        leaf: &<C as CommitmentScheme>::Output,
    ) -> Result<CommitmentMerklePath<C, H>, MerkleError> {
        let leaf_index = match self.leaves.iter().position(|l| l == leaf) {
            Some(index) => index,
            _ => return Err(MerkleError::InvalidLeaf),
        };

        let sibling_index = sibling(leaf_index);

        let leaf = leaf.clone();
        let sibling = self.leaves[sibling_index].clone();

        let leaves = match is_left_child(leaf_index) {
            true => (leaf, sibling),
            false => (sibling, leaf),
        };

        let inner_hashes = self.inner_hashes.clone();

        Ok(CommitmentMerklePath {
            leaves,
            inner_hashes,
            parameters: self.parameters.clone(),
        })
    }
}

/// Returns the index of the sibling leaf, given an index.
#[inline]
fn sibling(index: usize) -> usize {
    assert!(index < 4);
    match index {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        _ => unreachable!(),
    }
}

/// Returns true iff the given index represents a left child.
#[inline]
fn is_left_child(index: usize) -> bool {
    index % 2 == 0
}
