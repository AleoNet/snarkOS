#![deny(unused_import_braces, unused_qualifications, trivial_casts, trivial_numeric_casts)]
#![deny(unused_qualifications, variant_size_differences, stable_features, unreachable_pub)]
#![deny(non_shorthand_field_patterns, unused_attributes, unused_extern_crates)]
#![deny(
    renamed_and_removed_lints,
    stable_features,
    unused_allocation,
    unused_comparisons,
    bare_trait_objects
)]
#![deny(
    const_err,
    unused_must_use,
    unused_mut,
    unused_unsafe,
    private_in_public,
    unsafe_code
)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate derivative;

#[cfg(feature = "bls12_377")]
pub mod bls12_377;

#[cfg(feature = "edwards_bls12")]
pub mod edwards_bls12;

#[cfg(feature = "edwards_sw6")]
pub mod edwards_sw6;

#[cfg(feature = "sw6")]
pub mod sw6;

#[cfg(feature = "templates")]
pub mod templates;
