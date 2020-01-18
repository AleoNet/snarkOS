use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath};
use snarkos_models::{
    algorithms::CRH,
    curves::Field,
    gadgets::{
        algorithms::CRHGadget,
        r1cs::{ConstraintSystem, SynthesisError},
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
        assert_eq!(self.path.len(), P::HEIGHT - 1);
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{algorithms::crh::PedersenCRHGadget, curves::edwards_bls12::EdwardsBlsGadget};
    use snarkos_algorithms::{
        crh::{PedersenCRH, PedersenSize},
        merkle_tree::MerkleTree,
    };
    use snarkos_curves::edwards_bls12::{EdwardsAffine as Edwards, Fq};
    use snarkos_models::gadgets::{r1cs::TestConstraintSystem, utilities::uint8::UInt8};

    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;
    use std::rc::Rc;

    #[derive(Clone)]
    pub(super) struct Size;
    impl PedersenSize for Size {
        const NUM_WINDOWS: usize = 256;
        const WINDOW_SIZE: usize = 4;
    }

    type H = PedersenCRH<Edwards, Size>;
    type HG = PedersenCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

    #[derive(Clone)]
    struct EdwardsMerkleParameters(H);
    impl MerkleParameters for EdwardsMerkleParameters {
        type H = H;

        const HEIGHT: usize = 32;

        fn crh(&self) -> &Self::H {
            &self.0
        }
    }
    impl Default for EdwardsMerkleParameters {
        fn default() -> Self {
            let rng = &mut XorShiftRng::seed_from_u64(9174123u64);
            Self(H::setup(rng))
        }
    }

    type EdwardsMerkleTree = MerkleTree<EdwardsMerkleParameters>;

    fn generate_merkle_tree(leaves: &[[u8; 30]], use_bad_root: bool) -> () {
        let mut rng = XorShiftRng::seed_from_u64(9174123u64);

        let crh = Rc::new(H::setup(&mut rng));
        let crh_parameters = crh.parameters.clone();
        let tree = EdwardsMerkleTree::new(leaves).unwrap();
        let root = tree.root();
        let mut satisfied = true;
        for (i, leaf) in leaves.iter().enumerate() {
            let mut cs = TestConstraintSystem::<Fq>::new();
            let proof = tree.generate_proof(i, &leaf).unwrap();
            assert!(proof.verify(&root, &leaf).unwrap());

            // Allocate Merkle tree root
            let root = <HG as CRHGadget<H, _>>::OutputGadget::alloc(&mut cs.ns(|| format!("new_digest_{}", i)), || {
                if use_bad_root {
                    Ok(<H as CRH>::Output::default())
                } else {
                    Ok(root)
                }
            })
            .unwrap();

            let constraints_from_digest = cs.num_constraints();
            println!("constraints from digest: {}", constraints_from_digest);

            // Allocate Parameters for CRH
            let crh_parameters = <HG as CRHGadget<H, Fq>>::ParametersGadget::alloc(
                &mut cs.ns(|| format!("new_parameters_{}", i)),
                || Ok(crh_parameters.clone()),
            )
            .unwrap();

            let constraints_from_parameters = cs.num_constraints() - constraints_from_digest;
            println!("constraints from parameters: {}", constraints_from_parameters);

            // Allocate Leaf
            let leaf_g = UInt8::constant_vec(leaf);

            let constraints_from_leaf = cs.num_constraints() - constraints_from_parameters - constraints_from_digest;
            println!("constraints from leaf: {}", constraints_from_leaf);

            // Allocate Merkle tree path
            let cw =
                MerklePathGadget::<_, HG, _>::alloc(&mut cs.ns(|| format!("new_witness_{}", i)), || Ok(proof)).unwrap();

            let constraints_from_path =
                cs.num_constraints() - constraints_from_parameters - constraints_from_digest - constraints_from_leaf;
            println!("constraints from path: {}", constraints_from_path);
            let leaf_g: &[UInt8] = leaf_g.as_slice();
            cw.check_membership(
                &mut cs.ns(|| format!("new_witness_check_{}", i)),
                &crh_parameters,
                &root,
                &leaf_g,
            )
            .unwrap();
            if !cs.is_satisfied() {
                satisfied = false;
                println!("Unsatisfied constraint: {}", cs.which_is_unsatisfied().unwrap());
            }
            let setup_constraints =
                constraints_from_leaf + constraints_from_digest + constraints_from_parameters + constraints_from_path;
            println!("number of constraints: {}", cs.num_constraints() - setup_constraints);
        }

        assert!(satisfied);
    }

    #[test]
    fn good_root_test() {
        let mut leaves = Vec::new();
        for i in 0..4u8 {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree(&leaves, false);
    }

    #[should_panic]
    #[test]
    fn bad_root_test() {
        let mut leaves = Vec::new();
        for i in 0..4u8 {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree(&leaves, true);
    }
}
