use snarkos_models::curves::Field;

pub mod dummy_permutation;
pub mod hashchain;
pub mod poseidon;
pub mod sponge;

// Const Generics aren't stable, so the size parameter cannot be templated.
pub trait Permutation<F: Field> {
    fn permute(&self, state: &mut [F]);

    // pub fn print_soundness(&self);
}
