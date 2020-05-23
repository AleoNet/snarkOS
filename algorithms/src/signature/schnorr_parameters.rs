use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};
use snarkvm_errors::curves::ConstraintFieldError;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use digest::Digest;
use std::{
    fs::File,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    path::PathBuf,
};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "G: Group, D: Digest"),
    Debug(bound = "G: Group, D: Digest"),
    PartialEq(bound = "G: Group, D: Digest"),
    Eq(bound = "G: Group, D: Digest")
)]
pub struct SchnorrParameters<G: Group, D: Digest> {
    pub generator: G,
    pub salt: [u8; 32],
    pub _hash: PhantomData<D>,
}

impl<G: Group, D: Digest> ToBytes for SchnorrParameters<G, D> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.generator.write(&mut writer)?;
        self.salt.write(&mut writer)
    }
}

impl<G: Group, D: Digest> FromBytes for SchnorrParameters<G, D> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let generator: G = FromBytes::read(&mut reader)?;
        let salt: [u8; 32] = FromBytes::read(&mut reader)?;

        Ok(Self {
            generator,
            salt,
            _hash: PhantomData,
        })
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, D: Digest> ToConstraintField<F> for SchnorrParameters<G, D> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.generator.to_field_elements()
    }
}

impl<G: Group, D: Digest> SchnorrParameters<G, D> {
    /// Store the Schnorr parameters to a file at the given path.
    pub fn store(&self, path: &PathBuf) -> IoResult<()> {
        let mut file = File::create(path)?;
        let mut parameter_bytes = vec![];

        self.write(&mut parameter_bytes)?;
        file.write_all(&parameter_bytes)?;
        drop(file);

        Ok(())
    }

    /// Load the Schnorr parameters from a file at the given path.
    pub fn load(path: &PathBuf) -> IoResult<Self> {
        let mut file = File::open(path)?;
        Ok(Self::read(&mut file)?)
    }
}
