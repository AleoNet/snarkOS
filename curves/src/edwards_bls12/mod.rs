#![cfg_attr(nightly, doc(include = "../../documentation/the_aleo_curves/00_edwards_bls12.md"))]

pub mod fq;
#[doc(inline)]
pub use self::fq::*;

pub mod fr;
#[doc(inline)]
pub use self::fr::*;

pub mod parameters;
#[doc(inline)]
pub use self::parameters::*;

#[cfg(test)]
mod tests;
