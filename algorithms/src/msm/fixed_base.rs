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

use snarkos_models::curves::{FpParameters, PrimeField, ProjectiveCurve};
use snarkos_utilities::biginteger::BigInteger;

use rayon::prelude::*;

pub struct FixedBaseMSM;

impl FixedBaseMSM {
    pub fn get_mul_window_size(num_scalars: usize) -> usize {
        if num_scalars < 32 {
            3
        } else {
            (f64::from(num_scalars as u32)).ln().ceil() as usize
        }
    }

    pub fn get_window_table<T: ProjectiveCurve>(scalar_size: usize, window: usize, g: T) -> Vec<Vec<T>> {
        let in_window = 1 << window;
        let outerc = (scalar_size + window - 1) / window;
        let last_in_window = 1 << (scalar_size - (outerc - 1) * window);

        let mut multiples_of_g = vec![vec![T::zero(); in_window]; outerc];

        let mut g_outer = g;
        for (outer, m) in multiples_of_g.iter_mut().enumerate().take(outerc) {
            let mut g_inner = T::zero();
            let cur_in_window = if outer == outerc - 1 { last_in_window } else { in_window };
            for x in m.iter_mut().take(cur_in_window) {
                *x = g_inner;
                g_inner += &g_outer;
            }
            for _ in 0..window {
                g_outer.double_in_place();
            }
        }
        multiples_of_g
    }

    pub fn windowed_mul<T: ProjectiveCurve>(
        outerc: usize,
        window: usize,
        multiples_of_g: &[Vec<T>],
        scalar: &T::ScalarField,
    ) -> T {
        let mut scalar_val = scalar.into_repr().to_bits();
        scalar_val.reverse();

        let mut res = multiples_of_g[0][0];
        for outer in 0..outerc {
            let mut inner = 0usize;
            for i in 0..window {
                if outer * window + i < (<T::ScalarField as PrimeField>::Parameters::MODULUS_BITS as usize)
                    && scalar_val[outer * window + i]
                {
                    inner |= 1 << i;
                }
            }
            res += &multiples_of_g[outer][inner];
        }
        res
    }

    pub fn multi_scalar_mul<T: ProjectiveCurve>(
        scalar_size: usize,
        window: usize,
        table: &[Vec<T>],
        v: &[T::ScalarField],
    ) -> Vec<T> {
        let outerc = (scalar_size + window - 1) / window;
        assert!(outerc <= table.len());

        v.par_iter()
            .map(|e| Self::windowed_mul::<T>(outerc, window, table, e))
            .collect::<Vec<_>>()
    }
}
