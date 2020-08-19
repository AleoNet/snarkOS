// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::curves::templates::twisted_edwards::AffineGadget;
use snarkos_curves::edwards_sw6::{EdwardsParameters, Fq};
use snarkos_models::gadgets::curves::FpGadget;

pub type FqGadget = FpGadget<Fq>;

pub type EdwardsSWGadget = AffineGadget<EdwardsParameters, Fq, FqGadget>;

#[cfg(test)]
mod test {
    use super::EdwardsSWGadget;
    use crate::curves::templates::twisted_edwards::test::{edwards_constraint_costs, edwards_test};
    use snarkos_curves::edwards_sw6::{EdwardsParameters, Fq};
    use snarkos_models::gadgets::r1cs::TestConstraintSystem;

    #[test]
    fn edwards_constraint_costs_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();
        edwards_constraint_costs::<_, EdwardsParameters, EdwardsSWGadget, _>(&mut cs);
        assert!(cs.is_satisfied());
    }

    #[test]
    fn edwards_sw6_gadget_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();
        edwards_test::<_, EdwardsParameters, EdwardsSWGadget, _>(&mut cs);
        assert!(cs.is_satisfied());
    }
}
