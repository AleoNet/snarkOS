#![cfg_attr(nightly, doc(include = "../../documentation/the_aleo_curves/03_edwards_bw6.md"))]

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
