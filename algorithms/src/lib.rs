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

#![deny(unused_import_braces, trivial_casts, bare_trait_objects)]
#![deny(unused_qualifications, variant_size_differences, stable_features)]
#![deny(non_shorthand_field_patterns, unused_attributes)]
#![deny(renamed_and_removed_lints, unused_allocation, unused_comparisons)]
#![deny(const_err, unused_must_use, unused_mut, private_in_public)]
#![deny(unused_extern_crates, trivial_numeric_casts)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate snarkos_profiler;

pub mod commitment;
pub mod commitment_tree;
pub mod crh;
pub mod encoding;
pub mod encryption;
pub mod fft;
pub mod merkle_tree;
pub mod msm;
pub mod prf;
pub mod signature;
pub mod snark;
