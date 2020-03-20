//! An implementation of the [Groth-Maller][GM17] simulation extractable zkSNARK.
//! [GM17]: https://eprint.iacr.org/2017/540

use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::curves::pairing_engine::{PairingCurve, PairingEngine};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{self, Read, Result as IoResult, Write};

#[macro_use]
pub mod macros;

/// Reduce an R1CS instance to a *Square Arithmetic Program* instance.
pub mod r1cs_to_sap;

/// GM17 zkSNARK construction.
pub mod snark;
pub use self::snark::*;

/// Generate public parameters for the GM17 zkSNARK construction.
pub mod generator;
pub use self::generator::*;

/// Create proofs for the GM17 zkSNARK construction.
pub mod prover;
pub use self::prover::*;

/// Verify proofs for the GM17 zkSNARK construction.
pub mod verifier;
pub use self::verifier::*;

#[cfg(test)]
mod test;

/// A proof in the GM17 SNARK.
#[derive(Clone)]
pub struct Proof<E: PairingEngine> {
    pub a: E::G1Affine,
    pub b: E::G2Affine,
    pub c: E::G1Affine,
}

impl<E: PairingEngine> ToBytes for Proof<E> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        self.a.write(&mut writer)?;
        self.b.write(&mut writer)?;
        self.c.write(&mut writer)
    }
}

impl<E: PairingEngine> FromBytes for Proof<E> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> io::Result<Self> {
        let a: E::G1Affine = FromBytes::read(&mut reader)?;
        let b: E::G2Affine = FromBytes::read(&mut reader)?;
        let c: E::G1Affine = FromBytes::read(&mut reader)?;

        Ok(Self { a, b, c })
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
    pub fn write<W: Write>(&self, mut _writer: W) -> io::Result<()> {
        // TODO: implement serialization
        unimplemented!()
    }

    /// Deserialize the proof from bytes.
    pub fn read<R: Read>(mut _reader: R) -> io::Result<Self> {
        // TODO: implement serialization
        unimplemented!()
    }
}

/// A verification key in the GM17 SNARK.
#[derive(Clone)]
pub struct VerifyingKey<E: PairingEngine> {
    pub h_g2: E::G2Affine,
    pub g_alpha_g1: E::G1Affine,
    pub h_beta_g2: E::G2Affine,
    pub g_gamma_g1: E::G1Affine,
    pub h_gamma_g2: E::G2Affine,
    pub query: Vec<E::G1Affine>,
}

impl<E: PairingEngine> ToBytes for VerifyingKey<E> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.h_g2.write(&mut writer)?;
        self.g_alpha_g1.write(&mut writer)?;
        self.h_beta_g2.write(&mut writer)?;
        self.g_gamma_g1.write(&mut writer)?;
        self.h_gamma_g2.write(&mut writer)?;
        for q in &self.query {
            q.write(&mut writer)?;
        }
        Ok(())
    }
}

impl<E: PairingEngine> Default for VerifyingKey<E> {
    fn default() -> Self {
        Self {
            h_g2: E::G2Affine::default(),
            g_alpha_g1: E::G1Affine::default(),
            h_beta_g2: E::G2Affine::default(),
            g_gamma_g1: E::G1Affine::default(),
            h_gamma_g2: E::G2Affine::default(),
            query: Vec::new(),
        }
    }
}

impl<E: PairingEngine> PartialEq for VerifyingKey<E> {
    fn eq(&self, other: &Self) -> bool {
        self.h_g2 == other.h_g2
            && self.g_alpha_g1 == other.g_alpha_g1
            && self.h_beta_g2 == other.h_beta_g2
            && self.g_gamma_g1 == other.g_gamma_g1
            && self.h_gamma_g2 == other.h_gamma_g2
            && self.query == other.query
    }
}

impl<E: PairingEngine> VerifyingKey<E> {
    /// Serialize the verification key into bytes, for storage on disk
    /// or transmission over the network.
    pub fn write<W: Write>(&self, mut _writer: W) -> io::Result<()> {
        // TODO: implement serialization
        unimplemented!()
    }

    /// Deserialize the verification key from bytes.
    pub fn read<R: Read>(mut _reader: R) -> io::Result<Self> {
        // TODO: implement serialization
        unimplemented!()
    }
}

/// Full public (prover and verifier) parameters for the GM17 zkSNARK.
#[derive(Clone)]
pub struct Parameters<E: PairingEngine> {
    pub vk: VerifyingKey<E>,
    pub a_query: Vec<E::G1Affine>,
    pub b_query: Vec<E::G2Affine>,
    pub c_query_1: Vec<E::G1Affine>,
    pub c_query_2: Vec<E::G1Affine>,
    pub g_gamma_z: E::G1Affine,
    pub h_gamma_z: E::G2Affine,
    pub g_ab_gamma_z: E::G1Affine,
    pub g_gamma2_z2: E::G1Affine,
    pub g_gamma2_z_t: Vec<E::G1Affine>,
}

impl<E: PairingEngine> PartialEq for Parameters<E> {
    fn eq(&self, other: &Self) -> bool {
        self.vk == other.vk
            && self.a_query == other.a_query
            && self.b_query == other.b_query
            && self.c_query_1 == other.c_query_1
            && self.c_query_2 == other.c_query_2
            && self.g_gamma_z == other.g_gamma_z
            && self.h_gamma_z == other.h_gamma_z
            && self.g_ab_gamma_z == other.g_ab_gamma_z
            && self.g_gamma2_z2 == other.g_gamma2_z2
            && self.g_gamma2_z_t == other.g_gamma2_z_t
    }
}

impl<E: PairingEngine> Parameters<E> {
    /// Serialize the parameters to bytes.
    pub fn write<W: Write>(&self, mut _writer: W) -> io::Result<()> {
        // TODO: implement serialization
        unimplemented!()
    }

    /// Deserialize the public parameters from bytes.
    pub fn read<R: Read>(mut _reader: R, _checked: bool) -> io::Result<Self> {
        // TODO: implement serialization
        unimplemented!()
    }
}

/// Preprocessed verification key parameters that enable faster verification
/// at the expense of larger size in memory.
#[derive(Clone)]
pub struct PreparedVerifyingKey<E: PairingEngine> {
    pub vk: VerifyingKey<E>,
    pub g_alpha: E::G1Affine,
    pub h_beta: E::G2Affine,
    pub g_alpha_h_beta_ml: E::Fqk,
    pub g_gamma_pc: <E::G1Affine as PairingCurve>::Prepared,
    pub h_gamma_pc: <E::G2Affine as PairingCurve>::Prepared,
    pub h_pc: <E::G2Affine as PairingCurve>::Prepared,
    pub query: Vec<E::G1Affine>,
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

impl<E: PairingEngine> Default for PreparedVerifyingKey<E> {
    fn default() -> Self {
        Self {
            vk: VerifyingKey::default(),
            g_alpha: E::G1Affine::default(),
            h_beta: E::G2Affine::default(),
            g_alpha_h_beta_ml: E::Fqk::default(),
            g_gamma_pc: <E::G1Affine as PairingCurve>::Prepared::default(),
            h_gamma_pc: <E::G2Affine as PairingCurve>::Prepared::default(),
            h_pc: <E::G2Affine as PairingCurve>::Prepared::default(),
            query: Vec::new(),
        }
    }
}

impl<E: PairingEngine> ToBytes for PreparedVerifyingKey<E> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.vk.write(&mut writer)?;
        self.g_alpha.write(&mut writer)?;
        self.h_beta.write(&mut writer)?;
        self.g_alpha_h_beta_ml.write(&mut writer)?;
        self.g_gamma_pc.write(&mut writer)?;
        self.h_gamma_pc.write(&mut writer)?;
        self.h_pc.write(&mut writer)?;
        for q in &self.query {
            q.write(&mut writer)?;
        }
        Ok(())
    }
}

impl<E: PairingEngine> Parameters<E> {
    pub fn get_vk(&self, _: usize) -> Result<VerifyingKey<E>, SynthesisError> {
        Ok(self.vk.clone())
    }

    pub fn get_a_query(&self, num_inputs: usize) -> Result<(&[E::G1Affine], &[E::G1Affine]), SynthesisError> {
        Ok((&self.a_query[1..num_inputs], &self.a_query[num_inputs..]))
    }

    pub fn get_b_query(&self, num_inputs: usize) -> Result<(&[E::G2Affine], &[E::G2Affine]), SynthesisError> {
        Ok((&self.b_query[1..num_inputs], &self.b_query[num_inputs..]))
    }

    pub fn get_c_query_1(&self, num_inputs: usize) -> Result<(&[E::G1Affine], &[E::G1Affine]), SynthesisError> {
        Ok((&self.c_query_1[0..num_inputs], &self.c_query_1[num_inputs..]))
    }

    pub fn get_c_query_2(&self, num_inputs: usize) -> Result<(&[E::G1Affine], &[E::G1Affine]), SynthesisError> {
        Ok((&self.c_query_2[1..num_inputs], &self.c_query_2[num_inputs..]))
    }

    pub fn get_g_gamma_z(&self) -> Result<E::G1Affine, SynthesisError> {
        Ok(self.g_gamma_z)
    }

    pub fn get_h_gamma_z(&self) -> Result<E::G2Affine, SynthesisError> {
        Ok(self.h_gamma_z)
    }

    pub fn get_g_ab_gamma_z(&self) -> Result<E::G1Affine, SynthesisError> {
        Ok(self.g_ab_gamma_z)
    }

    pub fn get_g_gamma2_z2(&self) -> Result<E::G1Affine, SynthesisError> {
        Ok(self.g_gamma2_z2)
    }

    pub fn get_g_gamma2_z_t(&self, num_inputs: usize) -> Result<(&[E::G1Affine], &[E::G1Affine]), SynthesisError> {
        Ok((&self.g_gamma2_z_t[0..num_inputs], &self.g_gamma2_z_t[num_inputs..]))
    }

    pub fn get_a_query_full(&self) -> Result<&[E::G1Affine], SynthesisError> {
        Ok(&self.a_query)
    }

    pub fn get_b_query_full(&self) -> Result<&[E::G2Affine], SynthesisError> {
        Ok(&self.b_query)
    }

    pub fn get_c_query_1_full(&self) -> Result<&[E::G1Affine], SynthesisError> {
        Ok(&self.c_query_1)
    }

    pub fn get_c_query_2_full(&self) -> Result<&[E::G1Affine], SynthesisError> {
        Ok(&self.c_query_2)
    }

    pub fn get_g_gamma2_z_t_full(&self) -> Result<&[E::G1Affine], SynthesisError> {
        Ok(&self.g_gamma2_z_t)
    }
}
