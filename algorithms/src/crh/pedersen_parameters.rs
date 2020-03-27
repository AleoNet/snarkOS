use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{
    fs::File,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    path::PathBuf,
};

pub trait PedersenSize: Clone {
    const WINDOW_SIZE: usize;
    const NUM_WINDOWS: usize;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCRHParameters<G: Group, S: PedersenSize> {
    pub bases: Vec<Vec<G>>,
    _size: PhantomData<S>,
}

impl<G: Group, S: PedersenSize> ToBytes for PedersenCRHParameters<G, S> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.bases.len() as u32).write(&mut writer)?;
        for base in &self.bases {
            (base.len() as u32).write(&mut writer)?;
            for g in base {
                g.write(&mut writer)?;
            }
        }

        Ok(())
    }
}

impl<G: Group, S: PedersenSize> FromBytes for PedersenCRHParameters<G, S> {
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

        Ok(Self {
            bases,
            _size: PhantomData,
        })
    }
}

impl<G: Group, S: PedersenSize> PedersenCRHParameters<G, S> {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        let bases = (0..S::NUM_WINDOWS).map(|_| Self::base(S::WINDOW_SIZE, rng)).collect();
        Self {
            bases,
            _size: PhantomData,
        }
    }

    pub fn from(bases: Vec<Vec<G>>) -> Self {
        Self {
            bases,
            _size: PhantomData,
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

    /// Store the CRH parameters to a file at the given path.
    pub fn store(&self, path: &PathBuf) -> IoResult<()> {
        let mut file = File::create(path)?;
        let mut parameter_bytes = vec![];

        self.write(&mut parameter_bytes)?;
        file.write_all(&parameter_bytes)?;
        drop(file);

        Ok(())
    }

    /// Load the CRH parameters from a file at the given path.
    pub fn load(path: &PathBuf) -> IoResult<Self> {
        let mut file = File::open(path)?;
        Ok(Self::read(&mut file)?)
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F> for PedersenCRHParameters<G, S> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}
