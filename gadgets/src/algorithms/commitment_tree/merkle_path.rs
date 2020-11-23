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

use snarkos_algorithms::commitment_tree::CommitmentMerklePath;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::Field,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
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

pub struct CommitmentMerklePathGadget<
    C: CommitmentScheme,
    H: CRH,
    CG: CommitmentGadget<C, F>,
    HG: CRHGadget<H, F>,
    F: Field,
> {
    inner_hashes: (HG::OutputGadget, HG::OutputGadget),
    leaves: (CG::OutputGadget, CG::OutputGadget),
}

impl<C: CommitmentScheme, H: CRH, CG: CommitmentGadget<C, F>, HG: CRHGadget<H, F>, F: Field>
    CommitmentMerklePathGadget<C, H, CG, HG, F>
{
    pub fn check_membership<CS: ConstraintSystem<F>>(
        &self,
        cs: CS,
        parameters: &HG::ParametersGadget,
        root: &HG::OutputGadget,
        leaf: &CG::OutputGadget,
    ) -> Result<(), SynthesisError> {
        self.conditionally_check_membership(cs, parameters, root, leaf, &Boolean::Constant(true))
    }

    pub fn conditionally_check_membership<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        parameters: &HG::ParametersGadget,
        root: &HG::OutputGadget,
        leaf: &CG::OutputGadget,
        should_enforce: &Boolean,
    ) -> Result<(), SynthesisError> {
        // Check that the leaf is valid
        let left_leaf = &self.leaves.0;
        let right_leaf = &self.leaves.1;

        let leaf_is_left = AllocatedBit::alloc(&mut cs.ns(|| "leaf_is_left"), || Ok(leaf == left_leaf))?.into();

        let should_enforce_left = Boolean::and(cs.ns(|| "should_enforce_left"), should_enforce, &leaf_is_left)?;
        leaf.conditional_enforce_equal(&mut cs.ns(|| "check_leaf_is_left"), &leaf, &should_enforce_left)?;

        let should_enforce_right = Boolean::and(cs.ns(|| "should_enforce_right"), should_enforce, &leaf_is_left.not())?;
        leaf.conditional_enforce_equal(&mut cs.ns(|| "check_leaf_is_right"), &leaf, &should_enforce_right)?;

        // Check that the inner hash is valid
        let left_leaf_bytes = left_leaf.to_bytes(&mut cs.ns(|| "left_leaf_to_bytes"))?;
        let right_leaf_bytes = right_leaf.to_bytes(&mut cs.ns(|| "right_leaf_to_bytes"))?;
        let mut leaf_bytes = left_leaf_bytes;
        leaf_bytes.extend_from_slice(&right_leaf_bytes);

        let inner_hash = HG::check_evaluation_gadget(cs.ns(|| "inner_hash"), parameters, leaf_bytes)?;

        let left_inner_hash = &self.inner_hashes.0;
        let right_inner_hash = &self.inner_hashes.1;

        let inner_is_left =
            AllocatedBit::alloc(&mut cs.ns(|| "inner_is_left"), || Ok(&inner_hash == left_inner_hash))?.into();
        HG::OutputGadget::conditional_enforce_equal_or(
            &mut cs.ns(|| "check_inner_hash_is_left"),
            &inner_is_left,
            &inner_hash,
            left_inner_hash,
            right_inner_hash,
            should_enforce,
        )?;

        // Check that the root is valid
        let left_inner_hash_bytes = left_inner_hash.to_bytes(&mut cs.ns(|| "left_inner_hash_to_bytes"))?;
        let right_inner_hash_bytes = right_inner_hash.to_bytes(&mut cs.ns(|| "right_inner_hash_to_bytes"))?;
        let mut inner_hash_bytes = left_inner_hash_bytes;
        inner_hash_bytes.extend_from_slice(&right_inner_hash_bytes);

        let declared_root = HG::check_evaluation_gadget(cs.ns(|| "root_hash"), parameters, inner_hash_bytes)?;

        root.conditional_enforce_equal(&mut cs.ns(|| "check_root_is_valid"), &declared_root, should_enforce)?;

        Ok(())
    }
}

impl<C, H, CGadget, HGadget, F> AllocGadget<CommitmentMerklePath<C, H>, F>
    for CommitmentMerklePathGadget<C, H, CGadget, HGadget, F>
where
    C: CommitmentScheme,
    H: CRH,
    CGadget: CommitmentGadget<C, F>,
    HGadget: CRHGadget<H, F>,
    F: Field,
{
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<CommitmentMerklePath<C, H>>,
    {
        let commitment_merkle_path = value_gen()?.borrow().clone();

        let left_leaf = CGadget::OutputGadget::alloc(&mut cs.ns(|| "left leaf"), || {
            Ok(commitment_merkle_path.leaves.0.clone())
        })?;
        let right_leaf = CGadget::OutputGadget::alloc(&mut cs.ns(|| "right leaf"), || {
            Ok(commitment_merkle_path.leaves.1.clone())
        })?;

        let left_hash = HGadget::OutputGadget::alloc(&mut cs.ns(|| "left inner hash"), || {
            Ok(commitment_merkle_path.inner_hashes.0.clone())
        })?;
        let right_hash = HGadget::OutputGadget::alloc(&mut cs.ns(|| "right inner hash"), || {
            Ok(commitment_merkle_path.inner_hashes.1.clone())
        })?;

        let leaves = (left_leaf, right_leaf);
        let inner_hashes = (left_hash, right_hash);

        Ok(Self { inner_hashes, leaves })
    }

    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<CommitmentMerklePath<C, H>>,
    {
        let commitment_merkle_path = value_gen()?.borrow().clone();

        let left_leaf = CGadget::OutputGadget::alloc_input(&mut cs.ns(|| "left leaf"), || {
            Ok(commitment_merkle_path.leaves.0.clone())
        })?;
        let right_leaf = CGadget::OutputGadget::alloc_input(&mut cs.ns(|| "right leaf"), || {
            Ok(commitment_merkle_path.leaves.1.clone())
        })?;

        let left_hash = HGadget::OutputGadget::alloc_input(&mut cs.ns(|| "left inner hash"), || {
            Ok(commitment_merkle_path.inner_hashes.0.clone())
        })?;
        let right_hash = HGadget::OutputGadget::alloc_input(&mut cs.ns(|| "right inner hash"), || {
            Ok(commitment_merkle_path.inner_hashes.1.clone())
        })?;

        let leaves = (left_leaf, right_leaf);
        let inner_hashes = (left_hash, right_hash);

        Ok(Self { inner_hashes, leaves })
    }
}
