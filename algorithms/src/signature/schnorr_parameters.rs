use snarkos_errors::algorithms::Error;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};

use digest::Digest;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Clone(bound = "G: Group, H: Digest"))]
pub struct SchnorrParameters<G: Group, H: Digest> {
    pub generator: G,
    pub salt: [u8; 32],
    pub _hash: PhantomData<H>,
}

impl<F: Field, G: Group + ToConstraintField<F>, D: Digest> ToConstraintField<F> for SchnorrParameters<G, D> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, Error> {
        self.generator.to_field_elements()
    }
}
