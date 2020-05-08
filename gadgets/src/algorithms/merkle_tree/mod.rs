pub mod merkle_path;
pub use self::merkle_path::*;

pub mod masked_tree;
pub use self::masked_tree::*;

#[cfg(test)]
pub mod tests;

#[macro_export]
macro_rules! define_test_merkle_tree_with_height {
    ($struct_name:ident, $height:expr) => {
        use rand::{Rng, RngCore, SeedableRng};
        use rand_xorshift::XorShiftRng;
        use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTree};
        use snarkos_models::storage::Storage;
        use snarkos_utilities::bytes::{FromBytes, ToBytes};
        use std::{
            io::{Read, Result as IoResult, Write},
            path::PathBuf,
        };

        #[derive(Clone)]
        struct $struct_name(H);
        impl MerkleParameters for $struct_name {
            type H = H;

            const HEIGHT: usize = $height;

            fn setup<R: Rng>(rng: &mut R) -> Self {
                Self(H::setup(rng))
            }

            fn crh(&self) -> &Self::H {
                &self.0
            }

            fn parameters(&self) -> &<<Self as MerkleParameters>::H as CRH>::Parameters {
                self.crh().parameters()
            }
        }

        impl Storage for $struct_name {
            /// Store the SNARK proof to a file at the given path.
            fn store(&self, path: &PathBuf) -> IoResult<()> {
                self.0.store(path)
            }

            /// Load the SNARK proof from a file at the given path.
            fn load(path: &PathBuf) -> IoResult<Self> {
                Ok(Self(H::load(path)?))
            }
        }
        impl Default for $struct_name {
            fn default() -> Self {
                let rng = &mut XorShiftRng::seed_from_u64(9174123u64);
                Self(H::setup(rng))
            }
        }

        impl ToBytes for $struct_name {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                self.0.write(&mut writer)
            }
        }

        impl FromBytes for $struct_name {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let crh: H = FromBytes::read(&mut reader)?;

                Ok(Self(crh))
            }
        }
    };
}
