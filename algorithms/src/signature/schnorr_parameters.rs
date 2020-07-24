use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use digest::Digest;
use rand::Rng;
use std::{
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "G: Group, D: Digest"),
    Debug(bound = "G: Group, D: Digest"),
    PartialEq(bound = "G: Group, D: Digest"),
    Eq(bound = "G: Group, D: Digest")
)]
pub struct SchnorrParameters<G: Group, D: Digest> {
    pub generator_powers: Vec<G>,
    pub salt: [u8; 32],
    pub _hash: PhantomData<D>,
}

impl<G: Group, D: Digest> SchnorrParameters<G, D> {
    pub fn setup<R: Rng>(rng: &mut R, private_key_size_in_bits: usize) -> Self {
        // Round to the closest multiple of 64 to factor bit and byte encoding differences.
        assert!(private_key_size_in_bits < usize::MAX - 63);
        let num_powers = (private_key_size_in_bits + 63) & !63usize;
        Self {
            generator_powers: Self::generator(num_powers, rng),
            salt: rng.gen(),
            _hash: PhantomData,
        }
    }

    fn generator<R: Rng>(num_powers: usize, rng: &mut R) -> Vec<G> {
        let mut generator_powers = Vec::with_capacity(num_powers);
        let mut generator = G::rand(rng);
        for _ in 0..num_powers {
            generator_powers.push(generator);
            generator.double_in_place();
        }
        generator_powers
    }
}

impl<G: Group, D: Digest> ToBytes for SchnorrParameters<G, D> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.generator_powers.len() as u32).write(&mut writer)?;
        for g in &self.generator_powers {
            g.write(&mut writer)?;
        }
        self.salt.write(&mut writer)
    }
}

impl<G: Group, D: Digest> FromBytes for SchnorrParameters<G, D> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut generator_powers = vec![];
        let generator_powers_length: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..generator_powers_length {
            let g: G = FromBytes::read(&mut reader)?;
            generator_powers.push(g);
        }

        let salt: [u8; 32] = FromBytes::read(&mut reader)?;

        Ok(Self {
            generator_powers,
            salt,
            _hash: PhantomData,
        })
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, D: Digest> ToConstraintField<F> for SchnorrParameters<G, D> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}
