use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::io::{Read, Result as IoResult, Write};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCommitmentParameters<G: Group, S: PedersenSize> {
    pub bases: Vec<Vec<G>>,
    pub random_base: Vec<G>,
    pub crh: PedersenCRH<G, S>,
}

impl<G: Group, S: PedersenSize> PedersenCommitmentParameters<G, S> {
    pub fn setup<R: Rng>(rng: &mut R) -> Self {
        let bases = (0..S::NUM_WINDOWS)
            .map(|_| Self::base(S::WINDOW_SIZE, rng))
            .collect::<Vec<Vec<G>>>();
        let random_base = Self::base(S::WINDOW_SIZE, rng);
        let crh_parameters = PedersenCRHParameters::from(bases.clone());
        let crh = PedersenCRH::from(crh_parameters);
        Self {
            bases,
            random_base,
            crh,
        }
    }

    fn base<R: Rng>(num_powers: usize, rng: &mut R) -> Vec<G> {
        let mut powers = vec![];
        let mut base = G::rand(rng);
        for _ in 0..num_powers {
            powers.push(base);
            base.double_in_place();
        }
        powers
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F>
    for PedersenCommitmentParameters<G, S>
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}

impl<G: Group, S: PedersenSize> ToBytes for PedersenCommitmentParameters<G, S> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.bases.len() as u32).write(&mut writer)?;
        for base in &self.bases {
            (base.len() as u32).write(&mut writer)?;
            for g in base {
                g.write(&mut writer)?;
            }
        }

        (self.random_base.len() as u32).write(&mut writer)?;
        for g in &self.random_base {
            g.write(&mut writer)?;
        }

        self.crh.write(&mut writer)?;

        Ok(())
    }
}

impl<G: Group, S: PedersenSize> FromBytes for PedersenCommitmentParameters<G, S> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bases = vec![];

        let num_bases: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..num_bases {
            let mut base = vec![];

            let base_len: u32 = FromBytes::read(&mut reader)?;
            for _ in 0..base_len {
                let g: G = FromBytes::read(&mut reader)?;
                base.push(g);
            }
            bases.push(base);
        }

        let mut random_base = vec![];

        let random_base_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..random_base_len {
            let g: G = FromBytes::read(&mut reader)?;
            random_base.push(g);
        }

        let crh: PedersenCRH<G, S> = FromBytes::read(&mut reader)?;

        Ok(Self {
            bases,
            random_base,
            crh,
        })
    }
}
