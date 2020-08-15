// Compilation
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
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
// TODO (howardwu): Reenable after completing documentation in snarkOS-models.
// #![cfg_attr(nightly, warn(missing_docs))]
#![cfg_attr(nightly, doc(include = "../documentation/the_aleo_curves/00_the_aleo_curves.md"))]

#[macro_use]
extern crate derivative;

pub mod bls12_377;
pub mod bw6_761;
pub mod edwards_bls12;
pub mod edwards_sw6;
#[cfg(feature = "sw6")]
#[deprecated(since = "0.8.0", note = "Please use the `bw6_761` module instead")]
pub mod sw6;
pub mod templates;
