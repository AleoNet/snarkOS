use crate::algebraic_hash::hashing::{sponge::AlgebraicSponge, Permutation};
use snarkos_models::curves::PrimeField;
use snarkos_utilities::biginteger::BigInteger;

pub struct HashChain<F: PrimeField, P: Permutation<F> + Clone> {
    sponge: AlgebraicSponge<F, P>,
}

impl<F: PrimeField, P: Permutation<F> + Clone> HashChain<F, P> {
    pub fn new(sponge: AlgebraicSponge<F, P>) -> Self {
        let mut sponge_c = sponge.clone();
        sponge_c.reset();
        HashChain { sponge: sponge_c }
    }

    pub fn absorb(&mut self, elems: &[F]) {
        self.sponge.absorb(elems);
    }

    pub fn squeeze(&mut self, num_elements: usize) -> Vec<F> {
        self.sponge.squeeze(num_elements)
    }

    pub fn squeeze_ints(&mut self, num_elements: usize, bits_per_elem: usize) -> Vec<u64> {
        let squeezed_field_elems = self.sponge.squeeze(num_elements);

        let mut vec_of_bits: Vec<Vec<bool>> = squeezed_field_elems
            .into_iter()
            .map(|x| x.into_repr().to_bits())
            .collect();
        print!("{:?}", vec_of_bits);
        let mut squeezed_ints = Vec::new();
        for i in 0..num_elements {
            let mut cur = 0;
            vec_of_bits[i].reverse();
            for j in 0..bits_per_elem {
                cur <<= 1;
                cur += vec_of_bits[i][j] as u64;
            }
            squeezed_ints.push(cur);
        }

        squeezed_ints
    }
}
