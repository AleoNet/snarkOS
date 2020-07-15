use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::Field,
    gadgets::{curves::FieldGadget, r1cs::ConstraintSystem, utilities::boolean::Boolean},
};

pub fn mux<F: Field, FG: FieldGadget<F, F>, CS: ConstraintSystem<F>>(
    mut cs: CS,
    values: &[FG],
    location: &[Boolean],
) -> Result<FG, SynthesisError> {
    let N = values.len();
    let n = location.len();
    // Assert N is a power of 2, and n = log(N)
    assert!(N & (N - 1) == 0);
    assert!(1 << n == N);

    let mut cur_mux_values = values.to_vec();
    for i in (0..n) {
        let cur_size = 1 << (n - i);
        assert!(cur_mux_values.len() == cur_size);

        let mut next_mux_values = Vec::new();
        for j in (0..cur_size).step_by(2) {
            let cur = FG::conditionally_select(
                cs.ns(|| format!("mux layer {:?} index {:?}", i, j)),
                &location[n - 1 - i],
                // true case
                &cur_mux_values[j + 1],
                // false case
                &cur_mux_values[j],
            )?;
            next_mux_values.push(cur);
        }
        cur_mux_values = next_mux_values;
    }

    Ok(cur_mux_values[0].clone())
}

// Utility method for testing
pub fn int_to_constant_boolean_vec(index: u64, num_bits: u64) -> Vec<Boolean> {
    let mut location = Vec::new();
    for j in (0..num_bits).rev() {
        location.push(Boolean::constant(index & (1 << j) != 0));
    }

    location
}
