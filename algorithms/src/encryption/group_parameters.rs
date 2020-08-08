use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "G: Group"),
    Debug(bound = "G: Group"),
    PartialEq(bound = "G: Group"),
    Eq(bound = "G: Group")
)]
pub struct GroupEncryptionParameters<G: Group> {
    pub generator_powers: Vec<G>,
}

impl<G: Group> GroupEncryptionParameters<G> {
    pub fn setup<R: Rng>(rng: &mut R, private_key_size_in_bits: usize) -> Self {
        // Round to the closest multiple of 64 to factor bit and byte encoding differences.
        assert!(private_key_size_in_bits < std::usize::MAX - 63);
        let num_powers = (private_key_size_in_bits + 63) & !63usize;
        Self {
            generator_powers: Self::generator(num_powers, rng),
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

impl<G: Group> ToBytes for GroupEncryptionParameters<G> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.generator_powers.len() as u32).write(&mut writer)?;
        for g in &self.generator_powers {
            g.write(&mut writer)?;
        }
        Ok(())
    }
}

impl<G: Group> FromBytes for GroupEncryptionParameters<G> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut generator_powers = vec![];
        let generator_powers_length: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..generator_powers_length {
            let g: G = FromBytes::read(&mut reader)?;
            generator_powers.push(g);
        }

        Ok(Self { generator_powers })
    }
}

impl<F: Field, G: Group + ToConstraintField<F>> ToConstraintField<F> for GroupEncryptionParameters<G> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}
