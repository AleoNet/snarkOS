use snarkos_errors::algorithms::EncryptionError;
use snarkos_models::{
    algorithms::EncryptionScheme,
    curves::{AffineCurve, Field, Group, One, ProjectiveCurve, Zero},
};
use snarkos_utilities::{rand::UniformRand, to_bytes, FromBytes, ToBytes};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupEncryption<G: Group + ProjectiveCurve> {
    pub parameters: G,
}

impl<G: Group + ProjectiveCurve> EncryptionScheme for GroupEncryption<G> {
    type Ciphertext = Vec<G>;
    type Parameters = G;
    type Plaintext = Vec<G>;
    type PrivateKey = <G as Group>::ScalarField;
    type PublicKey = G;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: G::rand(rng),
        }
    }

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Self::PrivateKey {
        let keygen_time = start_timer!(|| "GroupEncryption::generate_private_key");
        let private_key = <G as Group>::ScalarField::rand(rng);
        end_timer!(keygen_time);

        private_key
    }

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Self::PublicKey {
        let keygen_time = start_timer!(|| "GroupEncryption::generate_public_key");
        let public_key = self.parameters.mul(&private_key);
        end_timer!(keygen_time);

        public_key
    }

    fn encrypt<R: Rng>(
        &self,
        public_key: &Self::PublicKey,
        message: &Self::Plaintext,
        rng: &mut R,
    ) -> Result<Self::Ciphertext, EncryptionError> {
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
                None => return Err(EncryptionError::MissingInverse),
            };

            // c_i <- h_i + m_i
            let c_i = h_i + m_i;

            ciphertext.push(c_i);
            i += &one;
        }

        Ok(ciphertext)
    }

    fn decrypt(
        &self,
        private_key: &Self::PrivateKey,
        ciphertext: &Self::Ciphertext,
    ) -> Result<Self::Plaintext, EncryptionError> {
        assert!(ciphertext.len() > 0);
        let c_0 = &ciphertext[0];

        let record_view_key = c_0.mul(&private_key);

        let affine = record_view_key.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        let z_bytes = to_bytes![affine.to_x_coordinate()]?;

        let z = <G as Group>::ScalarField::read(&z_bytes[..])?;

        let one = <G as Group>::ScalarField::one();
        let mut plaintext = vec![];
        let mut i = <G as Group>::ScalarField::one();

        for c_i in ciphertext.iter().skip(1) {
            // h_i <- 1 [/] (z [+] i) * record_view_key
            let h_i = match &(z + &i).inverse() {
                Some(val) => record_view_key.mul(val),
                None => return Err(EncryptionError::MissingInverse),
            };

            // m_i <- c_i - h_i
            let m_i = *c_i - &h_i;

            plaintext.push(m_i);
            i += &one;
        }

        Ok(plaintext)
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }
}

impl<G: Group + ProjectiveCurve> From<G> for GroupEncryption<G> {
    fn from(parameters: G) -> Self {
        Self { parameters }
    }
}
