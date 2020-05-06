use crate::algorithms::prf::blake2s_gadget;
use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::CRH,
    curves::PrimeField,
    gadgets::{
        algorithms::MaskedCRHGadget,
        r1cs::ConstraintSystem,
        utilities::{eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};

/// Checks a given `root` against a root computed given `leaves`. Uses a nonce to mask the
/// computation, to ensure hardness against amortization.
pub fn check_root<
    P: MerkleParameters,
    HG: MaskedCRHGadget<P::H, F>,
    F: PrimeField,
    TB: ToBytesGadget<F>,
    CS: ConstraintSystem<F>,
>(
    mut cs: CS,
    parameters: &HG::ParametersGadget,
    nonce: &[UInt8],
    root: &HG::OutputGadget,
    leaves: &[TB],
) -> Result<(), SynthesisError> {
    let nonce_bits = nonce.iter().flat_map(|b| b.into_bits_le()).collect::<Vec<_>>();
    let root_bytes = root.to_bytes(cs.ns(|| "convert root to bytes"))?;
    let root_bits = root_bytes.iter().flat_map(|b| b.into_bits_le()).collect::<Vec<_>>();
    // Derive a mask from the nonce and the root, such that the mask will have good randomness.
    // TODO(kobi): generalize to a generic hasher?
    let mask = blake2s_gadget(
        cs.ns(|| "derive mask from nonce || root"),
        &[nonce_bits, root_bits].concat(),
    )?;
    let mask_bytes = mask
        .iter()
        .enumerate()
        .map(|(i, m)| m.to_bytes(cs.ns(|| format!("mask part {} to bytes", i))))
        .collect::<Result<Vec<Vec<UInt8>>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    // Hash the leaves to get to the base level.
    let mut current_leaves = leaves
        .iter()
        .enumerate()
        .map(|(i, l)| {
            hash_leaf_gadget::<P::H, HG, F, _, _>(
                cs.ns(|| format!("hash leaf {}", i)),
                parameters,
                &l,
                &mask_bytes[..mask_bytes.len() / 2],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut level = 0;
    // Keep hashing pairs until there is only one element - the root.
    while current_leaves.len() != 1 {
        current_leaves = current_leaves
            .iter()
            .collect::<Vec<_>>()
            .chunks(2)
            .enumerate()
            .map(|(i, left_right)| {
                hash_inner_node_gadget::<P::H, HG, F, _, _>(
                    cs.ns(|| format!("hash left right {} on level {}", i, level)),
                    parameters,
                    &left_right[0],
                    &left_right[1],
                    &mask_bytes,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        level += 1;
    }

    // Hash the root.
    let computed_root = hash_leaf_gadget::<P::H, HG, F, _, _>(
        cs.ns(|| "hash root"),
        parameters,
        &current_leaves[0],
        &mask_bytes[..mask_bytes.len() / 2],
    )?;

    // Enforce the given root is equal to the computed root.
    root.enforce_equal(&mut cs.ns(|| "root_is_last"), &computed_root)
}

pub(crate) fn hash_inner_node_gadget<H, HG, F, TB, CS>(
    mut cs: CS,
    parameters: &HG::ParametersGadget,
    left_child: &TB,
    right_child: &TB,
    mask: &[UInt8],
) -> Result<HG::OutputGadget, SynthesisError>
where
    F: PrimeField,
    CS: ConstraintSystem<F>,
    H: CRH,
    HG: MaskedCRHGadget<H, F>,
    TB: ToBytesGadget<F>,
{
    let left_bytes = left_child.to_bytes(&mut cs.ns(|| "left_to_bytes"))?;
    let right_bytes = right_child.to_bytes(&mut cs.ns(|| "right_to_bytes"))?;
    let mut bytes = left_bytes;
    bytes.extend_from_slice(&right_bytes);

    HG::check_evaluation_gadget_masked(cs, parameters, &bytes, &mask)
}

pub(crate) fn hash_leaf_gadget<H, HG, F, TB, CS>(
    mut cs: CS,
    parameters: &HG::ParametersGadget,
    leaf: &TB,
    mask: &[UInt8],
) -> Result<HG::OutputGadget, SynthesisError>
where
    F: PrimeField,
    CS: ConstraintSystem<F>,
    H: CRH,
    HG: MaskedCRHGadget<H, F>,
    TB: ToBytesGadget<F>,
{
    let bytes = leaf.to_bytes(&mut cs.ns(|| "left_to_bytes"))?;
    HG::check_evaluation_gadget_masked(cs, parameters, &bytes, &mask)
}
