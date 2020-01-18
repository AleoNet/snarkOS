use crate::curves::templates::twisted_edwards::AffineGadget;
use snarkos_curves::edwards_bls12::{EdwardsParameters, Fq};
use snarkos_models::gadgets::curves::FpGadget;

pub type FqGadget = FpGadget<Fq>;
pub type EdwardsBlsGadget = AffineGadget<EdwardsParameters, Fq, FqGadget>;

#[cfg(test)]
mod test {
    use super::EdwardsBlsGadget;
    use crate::curves::templates::twisted_edwards::test::{edwards_constraint_costs, edwards_test};
    use snarkos_curves::edwards_bls12::{EdwardsParameters, Fq};
    use snarkos_models::gadgets::r1cs::TestConstraintSystem;

    #[test]
    fn edwards_constraint_costs_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();
        edwards_constraint_costs::<_, EdwardsParameters, EdwardsBlsGadget, _>(&mut cs);
        assert!(cs.is_satisfied());
    }

    #[test]
    fn edwards_bls12_gadget_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();
        edwards_test::<_, EdwardsParameters, EdwardsBlsGadget, _>(&mut cs);
        assert!(cs.is_satisfied());
    }
}
