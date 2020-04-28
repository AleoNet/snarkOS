#![deny(unused_import_braces, trivial_casts, bare_trait_objects)]
#![deny(unused_qualifications, variant_size_differences, stable_features)]
#![deny(non_shorthand_field_patterns, unused_attributes)]
#![deny(renamed_and_removed_lints, unused_allocation, unused_comparisons)]
#![deny(const_err, unused_must_use, unused_mut, private_in_public)]
#![deny(unreachable_pub, unused_extern_crates, trivial_numeric_casts)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate snarkos_profiler;

#[cfg(feature = "commitment")]
pub mod commitment;

#[cfg(feature = "crh")]
pub mod crh;

#[cfg(feature = "fft")]
pub mod fft;

#[cfg(feature = "merkle_tree")]
pub mod merkle_tree;

#[cfg(feature = "msm")]
pub mod msm;

#[cfg(feature = "prf")]
pub mod prf;

#[cfg(feature = "signature")]
pub mod signature;

#[cfg(feature = "snark")]
pub mod snark;
