use crate::algebraic_hash::hashing::Permutation;
use snarkos_models::curves::Field;

#[derive(Clone)]
pub struct DummyPermutation {}

impl<F: Field> Permutation<F> for DummyPermutation {
    fn permute(&self, state: &mut [F]) {
        for i in 0..state.len() {
            state[i] += &F::one();
        }
    }
}

#[derive(Clone)]
pub struct SeededDummyPermutation<F: Field> {
    pub seed: F,
}

impl<F: Field> Permutation<F> for SeededDummyPermutation<F> {
    fn permute(&self, state: &mut [F]) {
        let mut cur = self.seed;
        for i in 0..state.len() {
            state[i] += &cur;
            cur += &self.seed;
        }
    }
}
