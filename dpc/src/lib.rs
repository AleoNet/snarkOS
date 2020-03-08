#![deny(unused_import_braces, unused_qualifications, trivial_casts, trivial_numeric_casts)]
#![deny(
    single_use_lifetimes,
    unused_qualifications,
    variant_size_differences,
    stable_features,
    unreachable_pub
)]
#![deny(
    non_shorthand_field_patterns,
    unused_attributes,
    unused_imports,
    unused_extern_crates
)]
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
#![cfg_attr(feature = "clippy", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(inline_always))]
#![cfg_attr(feature = "clippy", allow(too_many_arguments))]
#![cfg_attr(feature = "clippy", allow(unreadable_literal))]
#![cfg_attr(feature = "clippy", allow(many_single_char_names))]
#![cfg_attr(feature = "clippy", allow(new_without_default_derive))]

#[macro_use]
extern crate snarkos_profiler;

#[macro_use]
extern crate derivative;

pub mod constraints;

pub(crate) mod dpc;
pub use crate::dpc::*;

pub mod ledger;
