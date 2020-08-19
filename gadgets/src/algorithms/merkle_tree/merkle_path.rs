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

use snarkos_algorithms::merkle_tree::MerklePath;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{MerkleParameters, CRH},
    curves::Field,
    gadgets::{
        algorithms::CRHGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::{ConditionalEqGadget, ConditionalOrEqualsGadget},
            ToBytesGadget,
        },
    },
};

use std::borrow::Borrow;

pub struct MerklePathGadget<P: MerkleParameters, HG: CRHGadget<P::H, F>, F: Field> {
    path: Vec<(HG::OutputGadget, HG::OutputGadget)>,
}

impl<P: MerkleParameters, HG: CRHGadget<P::H, F>, F: Field> MerklePathGadget<P, HG, F> {
    pub fn check_membership<CS: ConstraintSystem<F>>(
        &self,
        cs: CS,
        parameters: &HG::ParametersGadget,
        root: &HG::OutputGadget,
        leaf: impl ToBytesGadget<F>,
    ) -> Result<(), SynthesisError> {
        self.conditionally_check_membership(cs, parameters, root, leaf, &Boolean::Constant(true))
    }

    pub fn conditionally_check_membership<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        parameters: &HG::ParametersGadget,
        root: &HG::OutputGadget,
        leaf: impl ToBytesGadget<F>,
        should_enforce: &Boolean,
    ) -> Result<(), SynthesisError> {
        assert_eq!(self.path.len(), P::DEPTH);
        // Check that the hash of the given leaf matches the leaf hash in the membership
        // proof.
        let leaf_bits = leaf.to_bytes(&mut cs.ns(|| "leaf_to_bytes"))?;
        let leaf_hash = HG::check_evaluation_gadget(cs.ns(|| "check_evaluation_gadget"), parameters, &leaf_bits)?;

        // Check if leaf is one of the bottom-most siblings.
        let leaf_is_left =
            AllocatedBit::alloc(&mut cs.ns(|| "leaf_is_left"), || Ok(leaf_hash == self.path[0].0))?.into();
        HG::OutputGadget::conditional_enforce_equal_or(
            &mut cs.ns(|| "check_leaf_is_left"),
            &leaf_is_left,
            &leaf_hash,
            &self.path[0].0,
            &self.path[0].1,
            should_enforce,
        )?;

        // Check levels between leaf level and root.
        let mut previous_hash = leaf_hash;
        for (i, &(ref left_hash, ref right_hash)) in self.path.iter().enumerate() {
            // Check if the previous_hash matches the correct current hash.
            let previous_is_left = AllocatedBit::alloc(&mut cs.ns(|| format!("previous_is_left_{}", i)), || {
                Ok(&previous_hash == left_hash)
            })?
            .into();

            HG::OutputGadget::conditional_enforce_equal_or(
                &mut cs.ns(|| format!("check_equals_which_{}", i)),
                &previous_is_left,
                &previous_hash,
                left_hash,
                right_hash,
                should_enforce,
            )?;

            previous_hash = hash_inner_node_gadget::<P::H, HG, F, _>(
                &mut cs.ns(|| format!("hash_inner_node_{}", i)),
                parameters,
                left_hash,
                right_hash,
            )?;
        }

        root.conditional_enforce_equal(&mut cs.ns(|| "root_is_last"), &previous_hash, should_enforce)
    }
}

pub(crate) fn hash_inner_node_gadget<H, HG, F, CS>(
    mut cs: CS,
    parameters: &HG::ParametersGadget,
    left_child: &HG::OutputGadget,
    right_child: &HG::OutputGadget,
) -> Result<HG::OutputGadget, SynthesisError>
where
    F: Field,
    CS: ConstraintSystem<F>,
    H: CRH,
    HG: CRHGadget<H, F>,
{
    let left_bytes = left_child.to_bytes(&mut cs.ns(|| "left_to_bytes"))?;
    let right_bytes = right_child.to_bytes(&mut cs.ns(|| "right_to_bytes"))?;
    let mut bytes = left_bytes;
    bytes.extend_from_slice(&right_bytes);

    HG::check_evaluation_gadget(cs, parameters, &bytes)
}

impl<P, HGadget, F> AllocGadget<MerklePath<P>, F> for MerklePathGadget<P, HGadget, F>
where
    P: MerkleParameters,
    HGadget: CRHGadget<P::H, F>,
    F: Field,
{
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<MerklePath<P>>,
    {
        let mut path = Vec::new();
        for (i, &(ref l, ref r)) in value_gen()?.borrow().path.iter().enumerate() {
            let l_hash = HGadget::OutputGadget::alloc(&mut cs.ns(|| format!("l_child_{}", i)), || Ok(l.clone()))?;
            let r_hash = HGadget::OutputGadget::alloc(&mut cs.ns(|| format!("r_child_{}", i)), || Ok(r.clone()))?;
            path.push((l_hash, r_hash));
        }
        Ok(MerklePathGadget { path })
    }

    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<MerklePath<P>>,
    {
        let mut path = Vec::new();
        for (i, &(ref l, ref r)) in value_gen()?.borrow().path.iter().enumerate() {
            let l_hash = HGadget::OutputGadget::alloc_input(&mut cs.ns(|| format!("l_child_{}", i)), || Ok(l.clone()))?;
            let r_hash = HGadget::OutputGadget::alloc_input(&mut cs.ns(|| format!("r_child_{}", i)), || Ok(r.clone()))?;
            path.push((l_hash, r_hash));
        }

        Ok(MerklePathGadget { path })
    }
}
