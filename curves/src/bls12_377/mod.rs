#![cfg_attr(nightly, doc(include = "../../documentation/the_aleo_curves/02_bls12-377.md"))]

pub mod fr;
#[doc(inline)]
pub use self::fr::*;

pub mod fq;
#[doc(inline)]
pub use self::fq::*;

pub mod fq2;
#[doc(inline)]
pub use self::fq2::*;

pub mod fq6;
#[doc(inline)]
pub use self::fq6::*;

pub mod fq12;
#[doc(inline)]
pub use self::fq12::*;

pub mod g1;
#[doc(inline)]
pub use self::g1::*;

pub mod g2;
#[doc(inline)]
pub use self::g2::*;

pub mod parameters;
#[doc(inline)]
pub use self::parameters::*;

#[cfg(test)]
mod tests;
