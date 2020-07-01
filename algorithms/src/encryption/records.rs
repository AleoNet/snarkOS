use snarkos_errors::algorithms::EncryptionError;
use snarkos_models::{
    algorithms::EncryptionScheme,
    curves::{AffineCurve, Field, Group, One, ProjectiveCurve, Zero},
};
use snarkos_utilities::{rand::UniformRand, to_bytes, FromBytes, ToBytes};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordEncryption<G: Group + ProjectiveCurve> {
    pub parameters: G,
}

impl<G: Group + ProjectiveCurve> EncryptionScheme for RecordEncryption<G> {
    type Message = Vec<G>;
    type Output = Vec<G>;
    type PrivateKey = <G as Group>::ScalarField;
    type PublicKey = G;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: G::rand(rng),
        }
    }

    fn keygen<R: Rng>(&self, rng: &mut R) -> (Self::PrivateKey, Self::PublicKey) {
        let private_key = <G as Group>::ScalarField::rand(rng);

        let public_key = self.parameters.mul(&private_key);

        (private_key, public_key)
    }

    fn encrypt<R: Rng>(
        &self,
        public_key: &Self::PublicKey,
        message: &Self::Message,
        rng: &mut R,
    ) -> Result<Self::Output, EncryptionError> {
        let mut record_view_key = G::zero();
        let mut y = <G as Group>::ScalarField::zero();
        let mut z_bytes = vec![];

        while <G as Group>::ScalarField::read(&z_bytes[..]).is_err() {
            y = <G as Group>::ScalarField::rand(rng);
            record_view_key = public_key.mul(&y);

            let affine = record_view_key.into_affine();
            debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
            z_bytes = to_bytes![affine.to_x_coordinate()]?;
        }

        let z = <G as Group>::ScalarField::read(&z_bytes[..])?;

        let c_0 = self.parameters.mul(&y);

        let one = <G as Group>::ScalarField::one();
        let mut ciphertext = vec![c_0];
        let mut i = <G as Group>::ScalarField::one();

        for m_i in message {
            // h_i <- 1 [/] (z [+] i) * record_view_key
            let h_i = match &(z + &i).inverse() {
                Some(val) => record_view_key.mul(val),
                None => return Err(EncryptionError::Message("no inverse".into())),
            };

            // c_i <- h_i + m_i
            let c_i = h_i + m_i;

            ciphertext.push(c_i);
            i += &one;
        }

        Ok(ciphertext)
    }

    fn decrypt(&self, private_key: Self::PrivateKey, ciphertext: &Self::Output) -> Result<Vec<u8>, EncryptionError> {
        Ok(vec![])
    }
}
