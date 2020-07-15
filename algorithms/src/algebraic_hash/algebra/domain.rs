use snarkos_models::curves::PrimeField;

#[derive(Copy, Clone)]
pub struct Domain<F>
where
    F: PrimeField,
{
    pub gen: F,
    pub offset: F,
    pub dim: u64,
}

impl<F: PrimeField> Domain<F> {
    pub fn order(&self) -> u64 {
        1 << self.dim
    }

    // Returns g, g^2, ... g^{dim}
    pub fn powers_of_gen(&self, dim: usize) -> Vec<F> {
        let mut result = Vec::new();
        let mut cur = self.gen;
        for _ in 0..dim {
            result.push(cur);
            cur = cur * &cur;
        }
        result
    }
}
