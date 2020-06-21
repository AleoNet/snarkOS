//! This module contains an `EvaluationDomain` abstraction for
//! performing various kinds of polynomial arithmetic on top of
//! the scalar field.
//!
//! In pairing-based SNARKs like GM17, we need to calculate
//! a quotient polynomial over a target polynomial with roots
//! at distinct points associated with each constraint of the
//! constraint system. In order to be efficient, we choose these
//! roots to be the powers of a 2^n root of unity in the field.
//! This allows us to perform polynomial operations in O(n)
//! by performing an O(n log n) FFT over such a domain.

use crate::fft::{multicore::Worker, SparsePolynomial};
use snarkos_errors::serialization::SerializationError;
use snarkos_models::curves::{batch_inversion, FpParameters, PrimeField};
use snarkos_utilities::serialize::*;

use rand::Rng;
use rayon::prelude::*;
use std::fmt;

/// Defines a domain over which finite field (I)FFTs can be performed. Works
/// only for fields that have a large multiplicative subgroup of size that is
/// a power-of-2.
#[derive(Copy, Clone, Hash, Eq, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct EvaluationDomain<F: PrimeField> {
    /// The size of the domain.
    pub size: u64,
    /// `log_2(self.size)`.
    pub log_size_of_group: u32,
    /// Size of the domain as a field element.
    pub size_as_field_element: F,
    /// Inverse of the size in the field.
    pub size_inv: F,
    /// A generator of the subgroup.
    pub group_gen: F,
    /// Inverse of the generator of the subgroup.
    pub group_gen_inv: F,
    /// Multiplicative generator of the finite field.
    pub generator_inv: F,
}

impl<F: PrimeField> fmt::Debug for EvaluationDomain<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Multiplicative subgroup of size {}", self.size)
    }
}

impl<F: PrimeField> EvaluationDomain<F> {
    fn calculate_chunk_size(size: usize) -> usize {
        match size / rayon::current_num_threads() {
            0 => 1,
            chunk_size => chunk_size,
        }
    }

    /// Sample an element that is *not* in the domain.
    pub fn sample_element_outside_domain<R: Rng>(&self, rng: &mut R) -> F {
        let mut t = F::rand(rng);
        while self.evaluate_vanishing_polynomial(t).is_zero() {
            t = F::rand(rng);
        }
        t
    }

    /// Construct a domain that is large enough for evaluations of a polynomial
    /// having `num_coeffs` coefficients.
    pub fn new(num_coeffs: usize) -> Option<Self> {
        // Compute the size of our evaluation domain
        let size = num_coeffs.next_power_of_two() as u64;
        let log_size_of_group = size.trailing_zeros();

        if log_size_of_group >= F::Params::TWO_ADICITY {
            return None;
        }

        // Compute the generator for the multiplicative subgroup.
        // It should be 2^(log_size_of_group) root of unity.
        let mut group_gen = F::root_of_unity();
        for _ in log_size_of_group..F::Params::TWO_ADICITY {
            group_gen.square_in_place();
        }

        let size_as_bigint = F::BigInt::from(size);
        let size_as_field_element = F::from_repr(size_as_bigint);
        let size_inv = size_as_field_element.inverse()?;

        Some(EvaluationDomain {
            size,
            log_size_of_group,
            size_as_field_element,
            size_inv,
            group_gen,
            group_gen_inv: group_gen.inverse()?,
            generator_inv: F::multiplicative_generator().inverse()?,
        })
    }

    /// Return the size of a domain that is large enough for evaluations of a polynomial
    /// having `num_coeffs` coefficients.
    pub fn compute_size_of_domain(num_coeffs: usize) -> Option<usize> {
        let size = num_coeffs.next_power_of_two();
        if size.trailing_zeros() < F::Params::TWO_ADICITY {
            Some(size)
        } else {
            None
        }
    }

    /// Return the size of `self`.
    pub fn size(&self) -> usize {
        self.size as usize
    }

    /// Compute a FFT.
    pub fn fft(&self, coeffs: &[F]) -> Vec<F> {
        let mut coeffs = coeffs.to_vec();
        self.fft_in_place(&mut coeffs);
        coeffs
    }

    /// Compute a FFT, modifying the vector in place.
    pub fn fft_in_place(&self, coeffs: &mut Vec<F>) {
        coeffs.resize(self.size(), F::zero());
        best_fft(coeffs, &Worker::new(), self.group_gen, self.log_size_of_group)
    }

    /// Compute a IFFT.
    pub fn ifft(&self, evals: &[F]) -> Vec<F> {
        let mut evals = evals.to_vec();
        self.ifft_in_place(&mut evals);
        evals
    }

    /// Compute a IFFT, modifying the vector in place.
    #[inline]
    pub fn ifft_in_place(&self, evals: &mut Vec<F>) {
        evals.resize(self.size(), F::zero());
        best_fft(evals, &Worker::new(), self.group_gen_inv, self.log_size_of_group);
        evals.par_iter_mut().for_each(|val| *val *= &self.size_inv);
    }

    fn distribute_powers(coeffs: &mut Vec<F>, g: F) {
        Worker::new().scope(coeffs.len(), |scope, chunk| {
            for (i, v) in coeffs.chunks_mut(chunk).enumerate() {
                scope.spawn(move |_| {
                    let mut u = g.pow(&[(i * chunk) as u64]);
                    for v in v.iter_mut() {
                        *v *= &u;
                        u *= &g;
                    }
                });
            }
        });
    }

    /// Compute a FFT over a coset of the domain.
    pub fn coset_fft(&self, coeffs: &[F]) -> Vec<F> {
        let mut coeffs = coeffs.to_vec();
        self.coset_fft_in_place(&mut coeffs);
        coeffs
    }

    /// Compute a FFT over a coset of the domain, modifying the input vector
    /// in place.
    pub fn coset_fft_in_place(&self, coeffs: &mut Vec<F>) {
        Self::distribute_powers(coeffs, F::multiplicative_generator());
        self.fft_in_place(coeffs);
    }

    /// Compute a IFFT over a coset of the domain.
    pub fn coset_ifft(&self, evals: &[F]) -> Vec<F> {
        let mut evals = evals.to_vec();
        self.coset_ifft_in_place(&mut evals);
        evals
    }

    /// Compute a IFFT over a coset of the domain, modifying the input vector in place.
    pub fn coset_ifft_in_place(&self, evals: &mut Vec<F>) {
        self.ifft_in_place(evals);
        Self::distribute_powers(evals, self.generator_inv);
    }

    /// Evaluate all the lagrange polynomials defined by this domain at the point
    /// `tau`.
    pub fn evaluate_all_lagrange_coefficients(&self, tau: F) -> Vec<F> {
        // Evaluate all Lagrange polynomials
        let size = self.size as usize;
        let t_size = tau.pow(&[self.size]);
        let one = F::one();
        if t_size.is_one() {
            let mut u = vec![F::zero(); size];
            let mut omega_i = one;
            for i in 0..size {
                if omega_i == tau {
                    u[i] = one;
                    break;
                }
                omega_i *= &self.group_gen;
            }
            u
        } else {
            let mut l = (t_size - &one) * &self.size_inv;
            let mut r = one;
            let mut u = vec![F::zero(); size];
            let mut ls = vec![F::zero(); size];
            for i in 0..size {
                u[i] = tau - &r;
                ls[i] = l;
                l *= &self.group_gen;
                r *= &self.group_gen;
            }

            batch_inversion(u.as_mut_slice());
            u.par_iter_mut().zip(ls).for_each(|(tau_minus_r, l)| {
                *tau_minus_r = l * tau_minus_r;
            });
            u
        }
    }

    /// Return the sparse vanishing polynomial.
    pub fn vanishing_polynomial(&self) -> SparsePolynomial<F> {
        let coeffs = vec![(0, -F::one()), (self.size(), F::one())];
        SparsePolynomial::from_coefficients_vec(coeffs)
    }

    /// This evaluates the vanishing polynomial for this domain at tau.
    /// For multiplicative subgroups, this polynomial is `z(X) = X^self.size - 1`.
    pub fn evaluate_vanishing_polynomial(&self, tau: F) -> F {
        tau.pow(&[self.size]) - &F::one()
    }

    /// Return an iterator over the elements of the domain.
    pub fn elements(&self) -> Elements<F> {
        Elements {
            cur_elem: F::one(),
            cur_pow: 0,
            domain: *self,
        }
    }

    /// The target polynomial is the zero polynomial in our
    /// evaluation domain, so we must perform division over
    /// a coset.
    pub fn divide_by_vanishing_poly_on_coset_in_place(&self, evals: &mut [F]) {
        let i = self
            .evaluate_vanishing_polynomial(F::multiplicative_generator())
            .inverse()
            .unwrap();

        Worker::new().scope(evals.len(), |scope, chunk| {
            for evals in evals.chunks_mut(chunk) {
                scope.spawn(move |_| evals.iter_mut().for_each(|eval| *eval *= &i));
            }
        });
    }

    /// Given an index which assumes the first elements of this domain are the elements of
    /// another (sub)domain with size size_s,
    /// this returns the actual index into this domain.
    pub fn reindex_by_subdomain(&self, other: Self, index: usize) -> usize {
        assert!(self.size() >= other.size());
        // Let this subgroup be G, and the subgroup we're re-indexing by be S.
        // Since its a subgroup, the 0th element of S is at index 0 in G, the first element of S is at
        // index |G|/|S|, the second at 2*|G|/|S|, etc.
        // Thus for an index i that corresponds to S, the index in G is i*|G|/|S|
        let period = self.size() / other.size();
        if index < other.size() {
            index * period
        } else {
            // Let i now be the index of this element in G \ S
            // Let x be the number of elements in G \ S, for every element in S. Then x = (|G|/|S| - 1).
            // At index i in G \ S, the number of elements in S that appear before the index in G to which
            // i corresponds to, is floor(i / x) + 1.
            // The +1 is because index 0 of G is S_0, so the position is offset by at least one.
            // The floor(i / x) term is because after x elements in G \ S, there is one more element from S
            // that will have appeared in G.
            let i = index - other.size();
            let x = period - 1;
            i + (i / x) + 1
        }
    }

    /// Perform O(n) multiplication of two polynomials that are presented by their
    /// evaluations in the domain.
    /// Returns the evaluations of the product over the domain.
    #[must_use]
    pub fn mul_polynomials_in_evaluation_domain(&self, self_evals: &[F], other_evals: &[F]) -> Vec<F> {
        assert_eq!(self_evals.len(), other_evals.len());
        let mut result = self_evals.to_vec();
        let chunk_size = Self::calculate_chunk_size(self.size());
        result
            .par_chunks_mut(chunk_size)
            .zip(other_evals.par_chunks(chunk_size))
            .for_each(|(a, b)| {
                for (a, b) in a.iter_mut().zip(b) {
                    *a *= b;
                }
            });
        result
    }
}

fn best_fft<F: PrimeField>(a: &mut [F], worker: &Worker, omega: F, log_n: u32) {
    let log_cpus = worker.log_num_cpus();

    if log_n <= log_cpus {
        serial_fft(a, omega, log_n);
    } else {
        parallel_fft(a, worker, omega, log_n, log_cpus);
    }
}

pub(crate) fn serial_fft<F: PrimeField>(a: &mut [F], omega: F, log_n: u32) {
    #[inline]
    fn bitreverse(mut n: u32, l: u32) -> u32 {
        let mut r = 0;
        for _ in 0..l {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    let n = a.len() as u32;
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k, log_n);
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }

    let mut m = 1;
    for _ in 0..log_n {
        let w_m = omega.pow(&[(n / (2 * m)) as u64]);

        let mut k = 0;
        while k < n {
            let mut w = F::one();
            for j in 0..m {
                let mut t = a[(k + j + m) as usize];
                t *= &w;
                let mut tmp = a[(k + j) as usize];
                tmp -= &t;
                a[(k + j + m) as usize] = tmp;
                a[(k + j) as usize] += &t;
                w.mul_assign(&w_m);
            }

            k += 2 * m;
        }

        m *= 2;
    }
}

pub(crate) fn parallel_fft<F: PrimeField>(a: &mut [F], worker: &Worker, omega: F, log_n: u32, log_cpus: u32) {
    assert!(log_n >= log_cpus);

    let num_cpus = 1 << log_cpus;
    let log_new_n = log_n - log_cpus;
    let mut tmp = vec![vec![F::zero(); 1 << log_new_n]; num_cpus];
    let new_omega = omega.pow(&[num_cpus as u64]);

    worker.scope(0, |scope, _| {
        let a = &*a;

        for (j, tmp) in tmp.iter_mut().enumerate() {
            scope.spawn(move |_| {
                // Shuffle into a sub-FFT
                let omega_j = omega.pow(&[j as u64]);
                let omega_step = omega.pow(&[(j as u64) << log_new_n]);

                let mut elt = F::one();
                for i in 0..(1 << log_new_n) {
                    for s in 0..num_cpus {
                        let idx = (i + (s << log_new_n)) % (1 << log_n);
                        let mut t = a[idx];
                        t *= &elt;
                        tmp[i] += &t;
                        elt *= &omega_step;
                    }
                    elt *= &omega_j;
                }

                // Perform sub-FFT
                serial_fft(tmp, new_omega, log_new_n);
            });
        }
    });

    // TODO: does this hurt or help?
    worker.scope(a.len(), |scope, chunk| {
        let tmp = &tmp;

        for (idx, a) in a.chunks_mut(chunk).enumerate() {
            scope.spawn(move |_| {
                let mut idx = idx * chunk;
                let mask = (1 << log_cpus) - 1;
                for a in a {
                    *a = tmp[idx & mask][idx >> log_cpus];
                    idx += 1;
                }
            });
        }
    });
}

/// An iterator over the elements of the domain.
pub struct Elements<F: PrimeField> {
    cur_elem: F,
    cur_pow: u64,
    domain: EvaluationDomain<F>,
}

impl<F: PrimeField> Iterator for Elements<F> {
    type Item = F;

    fn next(&mut self) -> Option<F> {
        if self.cur_pow == self.domain.size {
            None
        } else {
            let cur_elem = self.cur_elem;
            self.cur_elem *= &self.domain.group_gen;
            self.cur_pow += 1;
            Some(cur_elem)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fft::EvaluationDomain;
    use snarkos_curves::bls12_377::Fr;
    use snarkos_models::curves::{Field, Zero};

    use rand::{thread_rng, Rng};

    #[test]
    fn vanishing_polynomial_evaluation() {
        let rng = &mut thread_rng();
        for coeffs in 0..10 {
            let domain = EvaluationDomain::<Fr>::new(coeffs).unwrap();
            let z = domain.vanishing_polynomial();
            for _ in 0..100 {
                let point = rng.gen();
                assert_eq!(z.evaluate(point), domain.evaluate_vanishing_polynomial(point))
            }
        }
    }

    #[test]
    fn vanishing_polynomial_vanishes_on_domain() {
        for coeffs in 0..1000 {
            let domain = EvaluationDomain::<Fr>::new(coeffs).unwrap();
            let z = domain.vanishing_polynomial();
            for point in domain.elements() {
                assert!(z.evaluate(point).is_zero())
            }
        }
    }

    #[test]
    fn size_of_elements() {
        for coeffs in 1..10 {
            let size = 1 << coeffs;
            let domain = EvaluationDomain::<Fr>::new(size).unwrap();
            let domain_size = domain.size();
            assert_eq!(domain_size, domain.elements().collect::<Vec<_>>().len());
        }
    }

    #[test]
    fn elements_contents() {
        for coeffs in 1..10 {
            let size = 1 << coeffs;
            let domain = EvaluationDomain::<Fr>::new(size).unwrap();
            for (i, element) in domain.elements().enumerate() {
                assert_eq!(element, domain.group_gen.pow([i as u64]));
            }
        }
    }
}
