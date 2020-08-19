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

use super::short_weierstrass_jacobian::{GroupAffine, GroupProjective};

use snarkos_utilities::{
    io::Cursor,
    rand::UniformRand,
    serialize::{CanonicalDeserialize, CanonicalSerialize},
};

use snarkos_models::curves::{
    pairing_engine::{AffineCurve, ProjectiveCurve},
    SWModelParameters,
    Zero,
};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub const ITERATIONS: usize = 10;

pub fn sw_tests<P: SWModelParameters>() {
    sw_curve_serialization_test::<P>();
    sw_from_random_bytes::<P>();
}

pub fn sw_curve_serialization_test<P: SWModelParameters>() {
    let buf_size = GroupAffine::<P>::zero().serialized_size();

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..10 {
        let a = GroupProjective::<P>::rand(&mut rng);
        let mut a = a.into_affine();
        {
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();

            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            a.y = -a.y;
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; buf_size - 1];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap_err();
        }

        {
            let serialized = vec![0; buf_size - 1];
            let mut cursor = Cursor::new(&serialized[..]);
            GroupAffine::<P>::deserialize(&mut cursor).unwrap_err();
        }

        {
            let mut serialized = vec![0; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();

            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            a.y = -a.y;
            let mut serialized = vec![0; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }
    }
}

pub fn sw_from_random_bytes<P: SWModelParameters>() {
    let buf_size = GroupAffine::<P>::zero().serialized_size();

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        let a = GroupProjective::<P>::rand(&mut rng);
        let a = a.into_affine();
        {
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();

            let mut cursor = Cursor::new(&serialized[..]);
            let p1 = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            let p2 = GroupAffine::<P>::from_random_bytes(&serialized).unwrap();
            assert_eq!(p1, p2);
        }
    }
}
