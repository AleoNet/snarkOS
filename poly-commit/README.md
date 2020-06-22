<h1 align="center">Univariate Polynomial Commitments</h1>

<p align="center">
    <a href="https://travis-ci.org/scipr-lab/poly-commit"><img src="https://travis-ci.org/scipr-lab/poly-commit.svg?branch=master"></a>
    <a href="https://github.com/scipr-lab/poly-commit/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SCIPR%20Lab-orange.svg"></a>
    <a href="https://github.com/scipr-lab/poly-commit/blob/master/LICENSE-APACHE"><img src="https://img.shields.io/badge/license-APACHE-blue.svg"></a>
   <a href="https://github.com/scipr-lab/poly-commit/blob/master/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

`poly-commit` is a Rust library that implements (univariate) *polynomial commitment schemes*. This library was initially developed as part of the [Marlin paper][marlin], and is released under the MIT License and the Apache v2 License (see [License](#license)).

**WARNING:** This is an academic prototype, and in particular has not received careful code review. This implementation is NOT ready for production use.

## Overview

A (univariate) polynomial commitment scheme is a cryptographic primitive that enables a party to commit to a univariate polynomial and then, later on, to reveal desired evaluations of the polynomial along with cryptographic proofs attesting to their correctness.

This library provides various constructions of polynomial commitment schemes. These constructions support committing to multiple polynomials at a time with differing degree bounds, batching multiple evaluation proofs for the same evaluation point into a single one, and batch verification of proofs.

The key properties satisfied by the polynomial commitment schemes are **succinctness**, **extractability**, and **hiding**. See [the Marlin paper][marlin] for definitions of these properties.


[kzg10]: http://cacr.uwaterloo.ca/techreports/2010/cacr2010-10.pdf

## Build guide

The library compiles on the `stable` toolchain of the Rust compiler. To install the latest version of Rust, first install `rustup` by following the instructions [here](https://rustup.rs/), or via your platform's package manager. Once `rustup` is installed, install the Rust toolchain by invoking:
```bash
rustup install stable
```

After that, use `cargo` (the standard Rust build tool) to build the library:
```bash
git clone https://github.com/scipr-lab/poly-commit.git
cd poly-commit
cargo build --release
```

This library comes with some unit and integration tests. Run these tests with:
```bash
cargo test
```

Lastly, this library is instrumented with profiling infrastructure that prints detailed traces of execution time. To enable this, compile with `cargo build --features print-trace`.

## License

This library is licensed under either of the following licenses, at your discretion.

 * [Apache License Version 2.0](LICENSE-APACHE)
 * [MIT License](LICENSE-MIT)

Unless you explicitly state otherwise, any contribution that you submit to this library shall be dual licensed as above (as defined in the Apache v2 License), without any additional terms or conditions.

[marlin]: https://ia.cr/2019/1047
[sonic]: https://ia.cr/2019/099
[aurora-light]: https://ia.cr/2019/601
[pcd-acc]: https://ia.cr/2020/499

## Reference papers

[Polynomial Commitments][kzg10]     
Aniket Kate, Gregory M. Zaverucha, Ian Goldberg     
ASIACRYPT 2010

[Sonic: Zero-Knowledge SNARKs from Linear-Size Universal and Updateable Structured Reference Strings][sonic]     
Mary Maller, Sean Bowe, Markulf Kohlweiss, Sarah Meiklejohn     
CCS 2019

[AuroraLight: Improved prover efficiency and SRS size in a Sonic-like system][aurora-light]     
Ariel Gabizon     
ePrint, 2019

[Marlin: Preprocessing zkSNARKs with Universal and Updatable SRS][marlin]     
Alessandro Chiesa, Yuncong Hu, Mary Maller, [Pratyush Mishra](https://www.github.com/pratyush), Noah Vesely, [Nicholas Ward](https://www.github.com/npwardberkeley)     
EUROCRYPT 2020

[Proof-Carrying Data from Accumulation Schemes][pcd-acc]     
Benedikt Bünz, Alessandro Chiesa, [Pratyush Mishra](https://www.github.com/pratyush), Nicholas Spooner     
ePrint, 2020


## Acknowledgements

This work was supported by: an Engineering and Physical Sciences Research Council grant; a Google Faculty Award; the RISELab at UC Berkeley; and donations from the Ethereum Foundation and the Interchain Foundation.
