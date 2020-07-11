//! An implementation of the [`Groth16`] zkSNARK.
//!
//! [`Groth16`]: https://eprint.iacr.org/2016/260.pdf
#![deny(unused_import_braces, unused_qualifications, trivial_casts)]
#![deny(trivial_numeric_casts, private_in_public, variant_size_differences)]
#![deny(stable_features, unreachable_pub, non_shorthand_field_patterns)]
#![deny(unused_attributes, unused_imports, unused_mut)]
#![deny(renamed_and_removed_lints, stable_features, unused_allocation)]
#![deny(unused_comparisons, bare_trait_objects, unused_must_use, const_err)]
#![forbid(unsafe_code)]

use snarkos_errors::serialization::SerializationError;
use snarkos_models::{
    curves::{AffineCurve, Field, PairingCurve, PairingEngine},
    gadgets::r1cs::{Index, LinearCombination},
};
use snarkos_utilities::{serialize::*, FromBytes, ToBytes};

use std::io::{self, Read, Result as IoResult, Write};

/// Reduce an R1CS instance to a *Quadratic Arithmetic Program* instance.
pub mod r1cs_to_qap;

/// Groth16 zkSNARK construction.
pub mod snark;
pub use self::snark::*;

/// Generate public parameters for the Groth16 zkSNARK construction.
pub mod generator;

/// Create proofs for the Groth16 zkSNARK construction.
pub mod prover;

/// Verify proofs for the Groth16 zkSNARK construction.
pub mod verifier;

#[cfg(test)]
mod test;

pub use self::{generator::*, prover::*, verifier::*};

/// A proof in the Groth16 SNARK.
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct Proof<E: PairingEngine> {
    pub a: E::G1Affine,
    pub b: E::G2Affine,
    pub c: E::G1Affine,
}

impl<E: PairingEngine> ToBytes for Proof<E> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.write(&mut writer)
    }
}

impl<E: PairingEngine> FromBytes for Proof<E> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        Self::read(&mut reader)
    }
}

impl<E: PairingEngine> PartialEq for Proof<E> {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b && self.c == other.c
    }
}

impl<E: PairingEngine> Default for Proof<E> {
    fn default() -> Self {
        Self {
            a: E::G1Affine::default(),
            b: E::G2Affine::default(),
            c: E::G1Affine::default(),
        }
    }
}

impl<E: PairingEngine> Proof<E> {
    /// Serialize the proof into bytes, for storage on disk or transmission
    /// over the network.
    pub fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.a.write(&mut writer)?;
        self.b.write(&mut writer)?;
        self.c.write(&mut writer)
    }

    /// Deserialize the proof from bytes.
    pub fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let a: E::G1Affine = FromBytes::read(&mut reader)?;
        let b: E::G2Affine = FromBytes::read(&mut reader)?;
        let c: E::G1Affine = FromBytes::read(&mut reader)?;

        Ok(Self { a, b, c })
    }
}

/// A verification key in the Groth16 SNARK.
#[derive(Clone, Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct VerifyingKey<E: PairingEngine> {
    pub alpha_g1: E::G1Affine,
    pub beta_g2: E::G2Affine,
    pub gamma_g2: E::G2Affine,
    pub delta_g2: E::G2Affine,
    pub gamma_abc_g1: Vec<E::G1Affine>,
}

impl<E: PairingEngine> ToBytes for VerifyingKey<E> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.write(&mut writer)
    }
}

impl<E: PairingEngine> FromBytes for VerifyingKey<E> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        Self::read(&mut reader)
    }
}

impl<E: PairingEngine> Default for VerifyingKey<E> {
    fn default() -> Self {
        Self {
            alpha_g1: E::G1Affine::default(),
            beta_g2: E::G2Affine::default(),
            gamma_g2: E::G2Affine::default(),
            delta_g2: E::G2Affine::default(),
            gamma_abc_g1: Vec::new(),
        }
    }
}

impl<E: PairingEngine> VerifyingKey<E> {
    /// Serialize the verification key into bytes, for storage on disk
    /// or transmission over the network.
    pub fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.alpha_g1.write(&mut writer)?;
        self.beta_g2.write(&mut writer)?;
        self.gamma_g2.write(&mut writer)?;
        self.delta_g2.write(&mut writer)?;
        (self.gamma_abc_g1.len() as u32).write(&mut writer)?;
        for g in &self.gamma_abc_g1 {
            g.write(&mut writer)?;
        }
        Ok(())
    }

    /// Deserialize the verification key from bytes.
    pub fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let alpha_g1: E::G1Affine = FromBytes::read(&mut reader)?;
        let beta_g2: E::G2Affine = FromBytes::read(&mut reader)?;
        let gamma_g2: E::G2Affine = FromBytes::read(&mut reader)?;
        let delta_g2: E::G2Affine = FromBytes::read(&mut reader)?;

        let gamma_abc_g1_len: u32 = FromBytes::read(&mut reader)?;
        let mut gamma_abc_g1: Vec<E::G1Affine> = vec![];
        for _ in 0..gamma_abc_g1_len {
            let gamma_abc_g1_element: E::G1Affine = FromBytes::read(&mut reader)?;
            gamma_abc_g1.push(gamma_abc_g1_element);
        }

        Ok(Self {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        })
    }
}

/// Full public (prover and verifier) parameters for the Groth16 zkSNARK.
#[derive(Clone, Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct Parameters<E: PairingEngine> {
    pub vk: VerifyingKey<E>,
    pub beta_g1: E::G1Affine,
    pub delta_g1: E::G1Affine,
    pub a_query: Vec<E::G1Affine>,
    pub b_g1_query: Vec<E::G1Affine>,
    pub b_g2_query: Vec<E::G2Affine>,
    pub h_query: Vec<E::G1Affine>,
    pub l_query: Vec<E::G1Affine>,
}

impl<E: PairingEngine> ToBytes for Parameters<E> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.write(&mut writer)
    }
}

impl<E: PairingEngine> FromBytes for Parameters<E> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        Self::read(&mut reader, false)
    }
}

impl<E: PairingEngine> Parameters<E> {
    /// Serialize the parameters to bytes.
    pub fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.vk.write(&mut writer)?;

        self.beta_g1.write(&mut writer)?;

        self.delta_g1.write(&mut writer)?;

        (self.a_query.len() as u32).write(&mut writer)?;
        for g in &self.a_query[..] {
            g.write(&mut writer)?;
        }

        (self.b_g1_query.len() as u32).write(&mut writer)?;
        for g in &self.b_g1_query[..] {
            g.write(&mut writer)?;
        }

        (self.b_g2_query.len() as u32).write(&mut writer)?;
        for g in &self.b_g2_query[..] {
            g.write(&mut writer)?;
        }

        (self.h_query.len() as u32).write(&mut writer)?;
        for g in &self.h_query[..] {
            g.write(&mut writer)?;
        }

        (self.l_query.len() as u32).write(&mut writer)?;
        for g in &self.l_query[..] {
            g.write(&mut writer)?;
        }

        Ok(())
    }

    /// Deserialize the public parameters from bytes.
    pub fn read<R: Read>(mut reader: R, checked: bool) -> IoResult<Self> {
        let read_g1_affine = |mut reader: &mut R| -> IoResult<E::G1Affine> {
            let g1_affine: E::G1Affine = FromBytes::read(&mut reader)?;

            if checked && !g1_affine.is_in_correct_subgroup_assuming_on_curve() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "point is not in the correct subgroup",
                ));
            }

            Ok(g1_affine)
        };

        let read_g2_affine = |mut reader: &mut R| -> IoResult<E::G2Affine> {
            let g2_affine: E::G2Affine = FromBytes::read(&mut reader)?;

            if checked && !g2_affine.is_in_correct_subgroup_assuming_on_curve() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "point is not in the correct subgroup",
                ));
            }

            Ok(g2_affine)
        };

        let vk = VerifyingKey::<E>::read(&mut reader)?;

        let beta_g1: E::G1Affine = FromBytes::read(&mut reader)?;

        let delta_g1: E::G1Affine = FromBytes::read(&mut reader)?;

        let mut a_query: Vec<E::G1Affine> = vec![];
        let mut b_g1_query: Vec<E::G1Affine> = vec![];
        let mut b_g2_query: Vec<E::G2Affine> = vec![];
        let mut h_query: Vec<E::G1Affine> = vec![];
        let mut l_query: Vec<E::G1Affine> = vec![];

        let a_query_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..a_query_len {
            a_query.push(read_g1_affine(&mut reader)?);
        }

        let b_g1_query_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..b_g1_query_len {
            b_g1_query.push(read_g1_affine(&mut reader)?);
        }

        let b_g2_query_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..b_g2_query_len {
            b_g2_query.push(read_g2_affine(&mut reader)?);
        }

        let h_query_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..h_query_len {
            h_query.push(read_g1_affine(&mut reader)?);
        }

        let l_query_len: u32 = FromBytes::read(&mut reader)?;
        for _ in 0..l_query_len {
            l_query.push(read_g1_affine(&mut reader)?);
        }

        Ok(Self {
            vk,
            beta_g1,
            delta_g1,
            a_query,
            b_g1_query,
            b_g2_query,
            h_query,
            l_query,
        })
    }
}

impl<E: PairingEngine> From<Parameters<E>> for VerifyingKey<E> {
    fn from(other: Parameters<E>) -> Self {
        other.vk
    }
}

impl<E: PairingEngine> From<Parameters<E>> for PreparedVerifyingKey<E> {
    fn from(other: Parameters<E>) -> Self {
        prepare_verifying_key(&other.vk)
    }
}

/// Preprocessed verification key parameters that enable faster verification
/// at the expense of larger size in memory.
#[derive(Clone, Debug)]
pub struct PreparedVerifyingKey<E: PairingEngine> {
    pub vk: VerifyingKey<E>,
    pub alpha_g1_beta_g2: E::Fqk,
    pub gamma_g2_neg_pc: <E::G2Affine as PairingCurve>::Prepared,
    pub delta_g2_neg_pc: <E::G2Affine as PairingCurve>::Prepared,
    pub gamma_abc_g1: Vec<E::G1Affine>,
}

impl<E: PairingEngine> From<PreparedVerifyingKey<E>> for VerifyingKey<E> {
    fn from(other: PreparedVerifyingKey<E>) -> Self {
        other.vk
    }
}

impl<E: PairingEngine> From<VerifyingKey<E>> for PreparedVerifyingKey<E> {
    fn from(other: VerifyingKey<E>) -> Self {
        prepare_verifying_key(&other)
    }
}

fn push_constraints<F: Field>(l: LinearCombination<F>, constraints: &mut [Vec<(F, Index)>], this_constraint: usize) {
    for (var, coeff) in l.as_ref() {
        match var.get_unchecked() {
            Index::Input(i) => constraints[this_constraint].push((*coeff, Index::Input(i))),
            Index::Aux(i) => constraints[this_constraint].push((*coeff, Index::Aux(i))),
        }
    }
}
