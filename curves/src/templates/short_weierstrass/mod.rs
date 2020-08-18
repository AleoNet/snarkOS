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

pub mod short_weierstrass_jacobian;
pub mod short_weierstrass_projective;
pub mod tests;

// Copied from https://github.com/scipr-lab/zexe/blob/4b3f08c6c0a08c5392ed8aa3fd3c32f28da402c4/algebra-core/src/curves/models/short_weierstrass_jacobian.rs#L160-L173.
#[macro_export]
macro_rules! impl_sw_from_random_bytes {
    () => {
        fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
            P::BaseField::from_random_bytes_with_flags(bytes).and_then(|(x, flags)| {
                let infinity_flag_mask = SWFlags::Infinity.u8_bitmask();
                let positive_flag_mask = SWFlags::PositiveY.u8_bitmask();
                // if x is valid and is zero and only the infinity flag is set, then parse this
                // point as infinity. For all other choices, get the original point.
                if x.is_zero() && flags == infinity_flag_mask {
                    Some(Self::zero())
                } else {
                    let is_positive = flags & positive_flag_mask != 0;
                    Self::from_x_coordinate(x, is_positive)
                }
            })
        }
    };
}
