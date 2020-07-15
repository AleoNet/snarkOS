use crate::algorithms::algebraic_hash::hashing::PermutationGadget;
use snarkos_algorithms::algebraic_hash::hashing::poseidon::PoseidonPermutation;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::Field,
    gadgets::{curves::FieldGadget, r1cs::ConstraintSystem},
};

pub struct PoseidonPermutationGadget<F: Field> {
    pub poseidon_permutation: PoseidonPermutation<F>,
}

impl<F: Field> PoseidonPermutationGadget<F> {
    fn new(poseidon_permutation: PoseidonPermutation<F>) -> Self {
        Self { poseidon_permutation }
    }

    fn apply_s_box<CS: ConstraintSystem<F>, FG: FieldGadget<F, F>>(
        &self,
        mut cs: CS,
        state: &mut [FG],
        is_full_round: bool,
    ) -> Result<(), SynthesisError> {
        // Full rounds apply the S Box (x^alpha) to every element of state
        if (is_full_round) {
            for i in 0..state.len() {
                state[i] = state[i].pow_by_constant(&mut cs.ns(|| format!("elem {:?}", i)), &[self
                    .poseidon_permutation
                    .alpha])?;
            }
        }
        // Partial rounds apply the S Box (x^alpha) to just the final element of state
        else {
            state[state.len() - 1] = state[state.len() - 1]
                .pow_by_constant(&mut cs.ns(|| "partial round"), &[self.poseidon_permutation.alpha])?;
        }

        Ok(())
    }

    fn apply_ark<CS: ConstraintSystem<F>, FG: FieldGadget<F, F>>(
        &self,
        mut cs: CS,
        state: &mut [FG],
        round_number: usize,
    ) {
        for i in 0..state.len() {
            state[i]
                .add_constant_in_place(&mut cs, &self.poseidon_permutation.ark[round_number][i])
                .unwrap();
        }
    }

    fn apply_mds<CS: ConstraintSystem<F>, FG: FieldGadget<F, F>>(
        &self,
        mut cs: CS,
        state: &mut [FG],
    ) -> Result<(), SynthesisError> {
        let mut new_state = Vec::new();
        for i in 0..state.len() {
            let mut cur = FG::zero(&mut cs)?;
            for j in 0..state.len() {
                let term = state[j].mul_by_constant(&mut cs, &self.poseidon_permutation.mds[i][j])?;
                cur.add_in_place(&mut cs, &term)?;
            }
            new_state.push(cur);
        }
        for i in 0..state.len() {
            state[i] = new_state[i].clone();
        }
        Ok(())
    }
}

impl<F: Field, FG: FieldGadget<F, F>> PermutationGadget<F, FG> for PoseidonPermutationGadget<F> {
    fn permute<CS: ConstraintSystem<F>>(&self, mut cs: CS, state: &mut [FG]) -> Result<(), SynthesisError> {
        let full_rounds_over_2 = self.poseidon_permutation.full_rounds / 2;
        for i in 0..full_rounds_over_2 {
            let mut cs_i = cs.ns(|| format!("Poseidon round {:?}", i));
            self.apply_ark(cs_i.ns(|| "ark"), state, i as usize);
            self.apply_s_box(cs_i.ns(|| "s_box"), state, true)?;
            self.apply_mds(cs_i.ns(|| "mds"), state)?;
        }

        for i in full_rounds_over_2..(full_rounds_over_2 + self.poseidon_permutation.partial_rounds) {
            let mut cs_i = cs.ns(|| format!("Poseidon round {:?}", i));
            // TODO: Optimize out most of the ARK / MDS work in partial rounds
            self.apply_ark(cs_i.ns(|| "ark"), state, i as usize);
            self.apply_s_box(cs_i.ns(|| "s_box"), state, false)?;
            self.apply_mds(cs_i.ns(|| "mds"), state)?;
        }

        for i in (full_rounds_over_2 + self.poseidon_permutation.partial_rounds)
            ..(self.poseidon_permutation.partial_rounds + self.poseidon_permutation.full_rounds)
        {
            let mut cs_i = cs.ns(|| format!("Poseidon round {:?}", i));
            self.apply_ark(cs_i.ns(|| "ark"), state, i as usize);
            self.apply_s_box(cs_i.ns(|| "s_box"), state, true)?;
            self.apply_mds(cs_i.ns(|| "mds"), state)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::algorithms::algebraic_hash::hashing::{poseidon::PoseidonPermutationGadget, PermutationGadget};
    use snarkos_algorithms::algebraic_hash::hashing::poseidon::libiop_near_mds_high_alpha_poseidon;
    use snarkos_curves::bw6_761::Fr;
    use snarkos_errors::gadgets::SynthesisError;
    use snarkos_models::gadgets::{
        curves::{FieldGadget, FpGadget},
        r1cs::TestConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget},
    };
    use std::str::FromStr;

    type FrGadget = FpGadget<Fr>;

    #[test]
    fn poseidon_test() -> Result<(), SynthesisError> {
        let mut cs = TestConstraintSystem::<Fr>::new();
        let poseidon = libiop_near_mds_high_alpha_poseidon::<Fr>();
        let mut state = vec![
            FrGadget::zero(&mut cs)?,
            FrGadget::zero(&mut cs)?,
            FrGadget::zero(&mut cs)?,
        ];
        let poseidon_gadget = PoseidonPermutationGadget::<Fr>::new(poseidon);
        poseidon_gadget.permute(&mut cs, &mut state).unwrap();
        let expected = Fr::from_str("80152274444821457455810455881201897082951372425736928808500357858200484271093001859415693954147682540005096304994").map_err(|_| ()).unwrap();
        let exp_gadg = FrGadget::alloc(&mut cs, || Ok(expected))?;
        state[0].enforce_equal(&mut cs, &exp_gadg).unwrap();
        if !cs.is_satisfied() {
            println!("unsatisfied: {}", cs.which_is_unsatisfied().unwrap());
        }
        assert!(cs.is_satisfied());
        Ok(())
    }
}
