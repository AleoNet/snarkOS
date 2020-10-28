use crate::{
    kzg10,
    BTreeMap,
    BTreeSet,
    BatchLCProof,
    Error,
    Evaluations,
    LabeledCommitment,
    LabeledPolynomial,
    LinearCombination,
    PCCommitterKey,
    PCRandomness,
    PCUniversalParams,
    Polynomial,
    PolynomialCommitment,
    QuerySet,
    String,
    ToString,
    Vec,
};

use snarkos_models::curves::{AffineCurve, Group, One, PairingCurve, PairingEngine, ProjectiveCurve, Zero};
use snarkos_utilities::rand::UniformRand;

use core::{convert::TryInto, marker::PhantomData};
use rand_core::RngCore;

mod data_structures;
pub use data_structures::*;

/// Polynomial commitment based on [[KZG10]][kzg], with degree enforcement and
/// batching taken from [[MBKM19, “Sonic”]][sonic] (more precisely, their
/// counterparts in [[Gabizon19, “AuroraLight”]][al] that avoid negative G1 powers).
/// The (optional) hiding property of the commitment scheme follows the approach
/// described in [[CHMMVW20, “Marlin”]][marlin].
///
/// [kzg]: http://cacr.uwaterloo.ca/techreports/2010/cacr2010-10.pdf
/// [sonic]: https://eprint.iacr.org/2019/099
/// [al]: https://eprint.iacr.org/2019/601
/// [marlin]: https://eprint.iacr.org/2019/1047
#[derive(Clone, Debug)]
pub struct SonicKZG10<E: PairingEngine> {
    _engine: PhantomData<E>,
}

impl<E: PairingEngine> SonicKZG10<E> {
    #[allow(clippy::too_many_arguments)]
    fn accumulate_elems<'a>(
        combined_comms: &mut BTreeMap<Option<usize>, E::G1Projective>,
        combined_witness: &mut E::G1Projective,
        combined_adjusted_witness: &mut E::G1Projective,
        vk: &VerifierKey<E>,
        commitments: impl IntoIterator<Item = &'a LabeledCommitment<Commitment<E>>>,
        point: E::Fr,
        values: impl IntoIterator<Item = E::Fr>,
        proof: &kzg10::Proof<E>,
        opening_challenge: E::Fr,
        randomizer: Option<E::Fr>,
    ) {
        let acc_time = start_timer!(|| "Accumulating elements");
        let mut curr_challenge = opening_challenge;

        // Keeps track of running combination of values
        let mut combined_values = E::Fr::zero();

        // Iterates through all of the commitments and accumulates common degree_bound elements in a BTreeMap
        for (labeled_comm, value) in commitments.into_iter().zip(values) {
            combined_values += &(value * &curr_challenge);

            let comm = labeled_comm.commitment();
            let degree_bound = labeled_comm.degree_bound();

            // Applying opening challenge and randomness (used in batch_checking)
            let mut comm_with_challenge: E::G1Projective = comm.0.mul(curr_challenge);

            if let Some(randomizer) = randomizer {
                comm_with_challenge = comm_with_challenge.mul(&randomizer);
            }

            // Accumulate values in the BTreeMap
            *combined_comms.entry(degree_bound).or_insert_with(E::G1Projective::zero) += &comm_with_challenge;
            curr_challenge *= &opening_challenge;
        }

        // Push expected results into list of elems. Power will be the negative of the expected power
        let mut witness: E::G1Projective = proof.w.into_projective();
        let mut adjusted_witness = vk.g.mul(combined_values) - &proof.w.mul(point);
        if let Some(random_v) = proof.random_v {
            adjusted_witness += &vk.gamma_g.mul(random_v);
        }

        if let Some(randomizer) = randomizer {
            witness = witness.mul(&randomizer);
            adjusted_witness = adjusted_witness.mul(&randomizer);
        }

        *combined_witness += &witness;
        *combined_adjusted_witness += &adjusted_witness;
        end_timer!(acc_time);
    }

    #[allow(clippy::type_complexity)]
    fn check_elems(
        combined_comms: BTreeMap<Option<usize>, E::G1Projective>,
        combined_witness: E::G1Projective,
        combined_adjusted_witness: E::G1Projective,
        vk: &VerifierKey<E>,
    ) -> Result<bool, Error> {
        let check_time = start_timer!(|| "Checking elems");
        let mut g1_projective_elems = Vec::with_capacity(combined_comms.len() + 2);
        let mut g2_prepared_elems = Vec::with_capacity(combined_comms.len() + 2);

        for (degree_bound, comm) in combined_comms.into_iter() {
            let shift_power = if let Some(degree_bound) = degree_bound {
                vk.get_shift_power(degree_bound)
                    .ok_or(Error::UnsupportedDegreeBound(degree_bound))?
            } else {
                vk.prepared_h.clone()
            };

            g1_projective_elems.push(comm);
            g2_prepared_elems.push(shift_power);
        }

        g1_projective_elems.push(-combined_adjusted_witness);
        g2_prepared_elems.push(vk.prepared_h.clone());

        g1_projective_elems.push(-combined_witness);
        g2_prepared_elems.push(vk.prepared_beta_h.clone());

        let g1_prepared_elems_iter = E::G1Projective::batch_normalization_into_affine(g1_projective_elems)
            .into_iter()
            .map(|a| a.prepare())
            .collect::<Vec<_>>();

        let g1_g2_prepared = g1_prepared_elems_iter.iter().zip(g2_prepared_elems.iter());
        let is_one: bool = E::product_of_pairings(g1_g2_prepared).is_one();
        end_timer!(check_time);
        Ok(is_one)
    }
}

impl<E: PairingEngine> PolynomialCommitment<E::Fr> for SonicKZG10<E> {
    type BatchProof = Vec<Self::Proof>;
    type Commitment = Commitment<E>;
    type CommitterKey = CommitterKey<E>;
    type Error = Error;
    type Proof = kzg10::Proof<E>;
    type Randomness = Randomness<E>;
    type UniversalParams = UniversalParams<E>;
    type VerifierKey = VerifierKey<E>;

    fn setup<R: RngCore>(max_degree: usize, rng: &mut R) -> Result<Self::UniversalParams, Self::Error> {
        kzg10::KZG10::setup(max_degree, true, rng).map_err(Into::into)
    }

    fn trim(
        pp: &Self::UniversalParams,
        supported_degree: usize,
        supported_hiding_bound: usize,
        enforced_degree_bounds: Option<&[usize]>,
    ) -> Result<(Self::CommitterKey, Self::VerifierKey), Self::Error> {
        let trim_time = start_timer!(|| "Trimming public parameters");
        let prepared_neg_powers_of_h = &pp.prepared_neg_powers_of_h;
        let max_degree = pp.max_degree();
        if supported_degree > max_degree {
            return Err(Error::TrimmingDegreeTooLarge);
        }

        let enforced_degree_bounds = enforced_degree_bounds.map(|bounds| {
            let mut v = bounds.to_vec();
            v.sort_unstable();
            v.dedup();
            v
        });

        let (shifted_powers_of_g, shifted_powers_of_gamma_g, degree_bounds_and_prepared_neg_powers_of_h) =
            if let Some(enforced_degree_bounds) = enforced_degree_bounds.as_ref() {
                if enforced_degree_bounds.is_empty() {
                    (None, None, None)
                } else {
                    let highest_enforced_degree_bound = *enforced_degree_bounds.last().unwrap();
                    if highest_enforced_degree_bound > supported_degree {
                        return Err(Error::UnsupportedDegreeBound(highest_enforced_degree_bound));
                    }

                    let lowest_shift_degree = max_degree - highest_enforced_degree_bound;

                    let shifted_ck_time = start_timer!(|| format!(
                        "Constructing `shifted_powers` of size {}",
                        max_degree - lowest_shift_degree + 1
                    ));

                    let shifted_powers_of_g = pp.powers_of_g[lowest_shift_degree..].to_vec();
                    let mut shifted_powers_of_gamma_g = BTreeMap::new();
                    // Also add degree 0.
                    let _max_gamma_g = pp.powers_of_gamma_g.keys().last().unwrap();
                    for degree_bound in enforced_degree_bounds {
                        let shift_degree = max_degree - degree_bound;
                        let mut powers_for_degree_bound =
                            Vec::with_capacity((max_degree + 2).saturating_sub(shift_degree));
                        for i in 0..=supported_hiding_bound + 1 {
                            // We have an additional degree in `powers_of_gamma_g` beyond `powers_of_g`.
                            if shift_degree + i < max_degree + 2 {
                                powers_for_degree_bound.push(pp.powers_of_gamma_g[&(shift_degree + i)]);
                            }
                        }
                        shifted_powers_of_gamma_g.insert(*degree_bound, powers_for_degree_bound);
                    }

                    end_timer!(shifted_ck_time);

                    let prepared_neg_powers_of_h_time = start_timer!(|| format!(
                        "Constructing `prepared_neg_powers_of_h` of size {}",
                        enforced_degree_bounds.len()
                    ));

                    let degree_bounds_and_prepared_neg_powers_of_h = enforced_degree_bounds
                        .iter()
                        .map(|bound| (*bound, prepared_neg_powers_of_h[&(max_degree - *bound)].clone()))
                        .collect();

                    end_timer!(prepared_neg_powers_of_h_time);

                    (
                        Some(shifted_powers_of_g),
                        Some(shifted_powers_of_gamma_g),
                        Some(degree_bounds_and_prepared_neg_powers_of_h),
                    )
                }
            } else {
                (None, None, None)
            };

        let powers_of_g = pp.powers_of_g[..=supported_degree].to_vec();
        let powers_of_gamma_g = (0..=supported_hiding_bound + 1)
            .map(|i| pp.powers_of_gamma_g[&i])
            .collect();

        let ck = CommitterKey {
            powers_of_g,
            powers_of_gamma_g,
            shifted_powers_of_g,
            shifted_powers_of_gamma_g,
            enforced_degree_bounds,
            max_degree,
        };

        let g = pp.powers_of_g[0];
        let h = pp.h;
        let beta_h = pp.beta_h;
        let gamma_g = pp.powers_of_gamma_g[&0];
        let prepared_h = (&pp.prepared_h).clone();
        let prepared_beta_h = (&pp.prepared_beta_h).clone();

        let vk = VerifierKey {
            g,
            gamma_g,
            h,
            beta_h,
            prepared_h,
            prepared_beta_h,
            degree_bounds_and_prepared_neg_powers_of_h,
            supported_degree,
            max_degree,
        };

        end_timer!(trim_time);
        Ok((ck, vk))
    }

    /// Outputs a commitment to `polynomial`.
    #[allow(clippy::type_complexity)]
    fn commit<'a>(
        ck: &Self::CommitterKey,
        polynomials: impl IntoIterator<Item = &'a LabeledPolynomial<'a, E::Fr>>,
        rng: Option<&mut dyn RngCore>,
    ) -> Result<(Vec<LabeledCommitment<Self::Commitment>>, Vec<Self::Randomness>), Self::Error> {
        let rng = &mut crate::optional_rng::OptionalRng(rng);
        let commit_time = start_timer!(|| "Committing to polynomials");
        let mut labeled_comms: Vec<LabeledCommitment<Self::Commitment>> = Vec::new();
        let mut randomness: Vec<Self::Randomness> = Vec::new();

        for labeled_polynomial in polynomials {
            let enforced_degree_bounds: Option<&[usize]> = ck.enforced_degree_bounds.as_deref();

            kzg10::KZG10::<E>::check_degrees_and_bounds(
                ck.supported_degree(),
                ck.max_degree,
                enforced_degree_bounds,
                &labeled_polynomial,
            )?;

            let polynomial = labeled_polynomial.polynomial();
            let degree_bound = labeled_polynomial.degree_bound();
            let hiding_bound = labeled_polynomial.hiding_bound();
            let label = labeled_polynomial.label();

            let commit_time = start_timer!(|| format!(
                "Polynomial {} of degree {}, degree bound {:?}, and hiding bound {:?}",
                label,
                polynomial.degree(),
                degree_bound,
                hiding_bound,
            ));

            let powers = if let Some(degree_bound) = degree_bound {
                ck.shifted_powers(degree_bound).unwrap()
            } else {
                ck.powers()
            };

            let (comm, rand) = kzg10::KZG10::commit(&powers, &polynomial, hiding_bound, Some(rng))?;

            labeled_comms.push(LabeledCommitment::new(label.to_string(), comm, degree_bound));
            randomness.push(rand);
            end_timer!(commit_time);
        }

        end_timer!(commit_time);
        Ok((labeled_comms, randomness))
    }

    fn open<'a>(
        ck: &Self::CommitterKey,
        labeled_polynomials: impl IntoIterator<Item = &'a LabeledPolynomial<'a, E::Fr>>,
        _commitments: impl IntoIterator<Item = &'a LabeledCommitment<Self::Commitment>>,
        point: E::Fr,
        opening_challenge: E::Fr,
        rands: impl IntoIterator<Item = &'a Self::Randomness>,
        _rng: Option<&mut dyn RngCore>,
    ) -> Result<Self::Proof, Self::Error>
    where
        Self::Randomness: 'a,
        Self::Commitment: 'a,
    {
        let mut combined_polynomial = Polynomial::zero();
        let mut combined_rand = kzg10::Randomness::empty();
        let mut curr_challenge = opening_challenge;

        for (polynomial, rand) in labeled_polynomials.into_iter().zip(rands) {
            let enforced_degree_bounds: Option<&[usize]> = ck.enforced_degree_bounds.as_deref();

            kzg10::KZG10::<E>::check_degrees_and_bounds(
                ck.supported_degree(),
                ck.max_degree,
                enforced_degree_bounds,
                &polynomial,
            )?;

            combined_polynomial += (curr_challenge, polynomial.polynomial());
            combined_rand += (curr_challenge, rand);
            curr_challenge *= &opening_challenge;
        }

        let proof_time = start_timer!(|| "Creating proof for polynomials");
        let proof = kzg10::KZG10::open(&ck.powers(), &combined_polynomial, point, &combined_rand)?;
        end_timer!(proof_time);

        Ok(proof)
    }

    fn check<'a, R: RngCore>(
        vk: &Self::VerifierKey,
        commitments: impl IntoIterator<Item = &'a LabeledCommitment<Self::Commitment>>,
        point: E::Fr,
        values: impl IntoIterator<Item = E::Fr>,
        proof: &Self::Proof,
        opening_challenge: E::Fr,
        _rng: &mut R,
    ) -> Result<bool, Self::Error>
    where
        Self::Commitment: 'a,
    {
        let check_time = start_timer!(|| "Checking evaluations");
        let mut combined_comms: BTreeMap<Option<usize>, E::G1Projective> = BTreeMap::new();
        let mut combined_witness: E::G1Projective = E::G1Projective::zero();
        let mut combined_adjusted_witness: E::G1Projective = E::G1Projective::zero();

        Self::accumulate_elems(
            &mut combined_comms,
            &mut combined_witness,
            &mut combined_adjusted_witness,
            vk,
            commitments,
            point,
            values,
            proof,
            opening_challenge,
            None,
        );

        let res = Self::check_elems(combined_comms, combined_witness, combined_adjusted_witness, vk);
        end_timer!(check_time);
        res
    }

    fn batch_check<'a, R: RngCore>(
        vk: &Self::VerifierKey,
        commitments: impl Iterator<Item = LabeledCommitment<Self::Commitment>>,
        query_set: &QuerySet<E::Fr>,
        values: &Evaluations<E::Fr>,
        proof: &Self::BatchProof,
        opening_challenge: E::Fr,
        rng: &mut R,
    ) -> Result<bool, Self::Error>
    where
        Self::Commitment: 'a,
    {
        let commitments: BTreeMap<_, _> = commitments.into_iter().map(|c| (c.label().to_owned(), c)).collect();
        let mut query_to_labels_map = BTreeMap::new();

        for (label, point) in query_set.iter() {
            let labels = query_to_labels_map.entry(point).or_insert_with(BTreeSet::new);
            labels.insert(label);
        }

        assert_eq!(proof.len(), query_to_labels_map.len());

        let mut randomizer = E::Fr::one();

        let mut combined_comms: BTreeMap<Option<usize>, E::G1Projective> = BTreeMap::new();
        let mut combined_witness: E::G1Projective = E::G1Projective::zero();
        let mut combined_adjusted_witness: E::G1Projective = E::G1Projective::zero();

        for ((query, labels), p) in query_to_labels_map.into_iter().zip(proof) {
            let mut comms_to_combine: Vec<&'_ LabeledCommitment<_>> = Vec::new();
            let mut values_to_combine = Vec::new();
            for label in labels.into_iter() {
                let commitment = commitments.get(label).ok_or(Error::MissingPolynomial {
                    label: label.to_string(),
                })?;

                let v_i = values.get(&(label.clone(), *query)).ok_or(Error::MissingEvaluation {
                    label: label.to_string(),
                })?;

                comms_to_combine.push(commitment);
                values_to_combine.push(*v_i);
            }

            Self::accumulate_elems(
                &mut combined_comms,
                &mut combined_witness,
                &mut combined_adjusted_witness,
                vk,
                comms_to_combine.into_iter(),
                *query,
                values_to_combine.into_iter(),
                p,
                opening_challenge,
                Some(randomizer),
            );

            randomizer = u128::rand(rng).into();
        }

        Self::check_elems(combined_comms, combined_witness, combined_adjusted_witness, vk)
    }

    fn open_combinations<'a>(
        ck: &Self::CommitterKey,
        lc_s: impl IntoIterator<Item = &'a LinearCombination<E::Fr>>,
        polynomials: impl IntoIterator<Item = &'a LabeledPolynomial<'a, E::Fr>>,
        commitments: impl IntoIterator<Item = &'a LabeledCommitment<Self::Commitment>>,
        query_set: &QuerySet<E::Fr>,
        opening_challenge: E::Fr,
        rands: impl IntoIterator<Item = &'a Self::Randomness>,
        rng: Option<&mut dyn RngCore>,
    ) -> Result<BatchLCProof<E::Fr, Self>, Self::Error>
    where
        Self::Randomness: 'a,
        Self::Commitment: 'a,
    {
        let label_map = polynomials
            .into_iter()
            .zip(rands)
            .zip(commitments)
            .map(|((p, r), c)| (p.label(), (p, r, c)))
            .collect::<BTreeMap<_, _>>();

        let mut lc_polynomials = Vec::new();
        let mut lc_randomness = Vec::new();
        let mut lc_commitments = Vec::new();
        let mut lc_info = Vec::new();

        for lc in lc_s {
            let lc_label = lc.label().clone();
            let mut poly = Polynomial::zero();
            let mut degree_bound = None;
            let mut hiding_bound = None;
            let mut randomness = Self::Randomness::empty();
            let mut comm = E::G1Projective::zero();

            let num_polys = lc.len();
            for (coeff, label) in lc.iter().filter(|(_, l)| !l.is_one()) {
                let label: &String = label.try_into().expect("cannot be one!");
                let &(cur_poly, cur_rand, curr_comm) = label_map.get(label).ok_or(Error::MissingPolynomial {
                    label: label.to_string(),
                })?;

                if num_polys == 1 && cur_poly.degree_bound().is_some() {
                    assert!(coeff.is_one(), "Coefficient must be one for degree-bounded equations");
                    degree_bound = cur_poly.degree_bound();
                } else if cur_poly.degree_bound().is_some() {
                    eprintln!("Degree bound when number of equations is non-zero");
                    return Err(Self::Error::EquationHasDegreeBounds(lc_label));
                }

                // Some(_) > None, always.
                hiding_bound = core::cmp::max(hiding_bound, cur_poly.hiding_bound());
                poly += (*coeff, cur_poly.polynomial());
                randomness += (*coeff, cur_rand);
                comm += &curr_comm.commitment().0.into_projective().mul(coeff);
            }

            let lc_poly = LabeledPolynomial::new_owned(lc_label.clone(), poly, degree_bound, hiding_bound);
            lc_polynomials.push(lc_poly);
            lc_randomness.push(randomness);
            lc_commitments.push(comm);
            lc_info.push((lc_label, degree_bound));
        }

        let comms: Vec<Self::Commitment> = E::G1Projective::batch_normalization_into_affine(lc_commitments)
            .into_iter()
            .map(kzg10::Commitment::<E>)
            .collect();

        let lc_commitments = lc_info
            .into_iter()
            .zip(comms)
            .map(|((label, d), c)| LabeledCommitment::new(label, c, d))
            .collect::<Vec<_>>();

        let proof = Self::batch_open(
            ck,
            lc_polynomials.iter(),
            lc_commitments.iter(),
            &query_set,
            opening_challenge,
            lc_randomness.iter(),
            rng,
        )?;
        Ok(BatchLCProof { proof, evals: None })
    }

    /// Checks that `values` are the true evaluations at `query_set` of the polynomials
    /// committed in `labeled_commitments`.
    fn check_combinations<'a, R: RngCore>(
        vk: &Self::VerifierKey,
        lc_s: impl IntoIterator<Item = &'a LinearCombination<E::Fr>>,
        commitments: impl Iterator<Item = LabeledCommitment<Self::Commitment>>,
        query_set: &QuerySet<E::Fr>,
        evaluations: &Evaluations<E::Fr>,
        proof: &BatchLCProof<E::Fr, Self>,
        opening_challenge: E::Fr,
        rng: &mut R,
    ) -> Result<bool, Self::Error>
    where
        Self::Commitment: 'a,
    {
        let BatchLCProof { proof, .. } = proof;
        let label_comm_map = commitments
            .into_iter()
            .map(|c| (c.label().to_owned(), c))
            .collect::<BTreeMap<_, _>>();

        let mut lc_commitments = Vec::new();
        let mut lc_info = Vec::new();
        let mut evaluations = evaluations.clone();
        for lc in lc_s {
            let lc_label = lc.label().clone();
            let num_polys = lc.len();

            let mut degree_bound = None;
            let mut combined_comm = E::G1Projective::zero();

            for (coeff, label) in lc.iter() {
                if label.is_one() {
                    for (&(ref label, _), ref mut eval) in evaluations.iter_mut() {
                        if label == &lc_label {
                            **eval -= coeff;
                        }
                    }
                } else {
                    let label: String = label.to_owned().try_into().unwrap();
                    let cur_comm = label_comm_map.get(&label).ok_or(Error::MissingPolynomial {
                        label: label.to_string(),
                    })?;

                    if num_polys == 1 && cur_comm.degree_bound().is_some() {
                        assert!(coeff.is_one(), "Coefficient must be one for degree-bounded equations");
                        degree_bound = cur_comm.degree_bound();
                    } else if cur_comm.degree_bound().is_some() {
                        return Err(Self::Error::EquationHasDegreeBounds(lc_label));
                    }
                    combined_comm += &cur_comm.commitment().0.mul(*coeff);
                }
            }

            lc_commitments.push(combined_comm);
            lc_info.push((lc_label, degree_bound));
        }

        let comms = E::G1Projective::batch_normalization_into_affine(lc_commitments)
            .into_iter()
            .map(kzg10::Commitment);

        let lc_commitments = lc_info
            .into_iter()
            .zip(comms)
            .map(|((label, d), c)| LabeledCommitment::new(label, c, d));

        Self::batch_check(
            vk,
            lc_commitments,
            &query_set,
            &evaluations,
            proof,
            opening_challenge,
            rng,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_camel_case_types)]

    use super::SonicKZG10;
    use snarkos_curves::bls12_377::Bls12_377;

    type PC<E> = SonicKZG10<E>;
    type PC_Bls12_377 = PC<Bls12_377>;

    #[test]
    fn single_poly_test() {
        use crate::tests::*;
        single_poly_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn quadratic_poly_degree_bound_multiple_queries_test() {
        use crate::tests::*;
        quadratic_poly_degree_bound_multiple_queries_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn linear_poly_degree_bound_test() {
        use crate::tests::*;
        linear_poly_degree_bound_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn single_poly_degree_bound_test() {
        use crate::tests::*;
        single_poly_degree_bound_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn single_poly_degree_bound_multiple_queries_test() {
        use crate::tests::*;
        single_poly_degree_bound_multiple_queries_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn two_polys_degree_bound_single_query_test() {
        use crate::tests::*;
        two_polys_degree_bound_single_query_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
    }

    #[test]
    fn full_end_to_end_test() {
        use crate::tests::*;
        full_end_to_end_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }

    #[test]
    fn single_equation_test() {
        use crate::tests::*;
        single_equation_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }

    #[test]
    fn two_equation_test() {
        use crate::tests::*;
        two_equation_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }

    #[test]
    fn two_equation_degree_bound_test() {
        use crate::tests::*;
        two_equation_degree_bound_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }

    #[test]
    fn full_end_to_end_equation_test() {
        use crate::tests::*;
        full_end_to_end_equation_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }

    #[test]
    #[should_panic]
    fn bad_degree_bound_test() {
        use crate::tests::*;
        bad_degree_bound_test::<_, PC_Bls12_377>().expect("test failed for bls12-377");
        println!("Finished bls12-377");
    }
}
