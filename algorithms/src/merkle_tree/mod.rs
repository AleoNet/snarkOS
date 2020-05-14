pub mod merkle_parameters;
pub use self::merkle_parameters::*;

pub mod merkle_path;
pub use self::merkle_path::*;

pub mod merkle_tree;
pub use self::merkle_tree::*;

#[cfg(test)]
pub mod tests;

#[macro_export]
/// Defines a Merkle Tree using the provided hash and height.
macro_rules! define_merkle_tree_parameters {
    ($struct_name:ident, $hash:ty, $height:expr) => {
        use rand::Rng;
        use snarkos_models::{algorithms::crh::CRH, storage::Storage};
        use snarkos_utilities::bytes::{FromBytes, ToBytes};
        use std::{
            io::{Read, Result as IoResult, Write},
            path::PathBuf,
        };
        use $crate::merkle_tree::{MerkleParameters, MerkleTree};

        #[derive(Clone, PartialEq, Eq)]
        pub struct $struct_name($hash);

        impl MerkleParameters for $struct_name {
            type H = $hash;

            const HEIGHT: usize = $height;

            fn setup<R: Rng>(rng: &mut R) -> Self {
                Self(Self::H::setup(rng))
            }

            fn crh(&self) -> &Self::H {
                &self.0
            }

            fn parameters(&self) -> &<Self::H as CRH>::Parameters {
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
                Ok(Self(<Self as MerkleParameters>::H::load(path)?))
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
                let crh: <Self as MerkleParameters>::H = FromBytes::read(&mut reader)?;

                Ok(Self(crh))
            }
        }
    };
}
