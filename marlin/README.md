# Marlin

`marlin` is a Rust library that implements a
<p align="center">
<b>preprocessing zkSNARK for R1CS</b><br>
with<br>
<b>universal and updatable SRS</b>
</p>

This library was initially developed as part of the [Marlin paper][marlin].

## Overview

A zkSNARK with **preprocessing** achieves succinct verification for arbitrary computations, as opposed to only for structured computations. Informally, in an offline phase, one can preprocess the desired computation to produce a short summary of it; subsequently, in an online phase, this summary can be used to check any number of arguments relative to this computation.

The preprocessing zkSNARKs in this library rely on a structured reference string (SRS), which contains system parameters required by the argument system to produce/validate arguments. The SRS in this library is **universal**, which means that it supports (deterministically) preprocessing any computation up to a given size bound. The SRS is also **updatable**, which means that anyone can contribute a fresh share of randomness to it, which facilitates deployments in the real world.

The construction in this library follows the methodology introduced in the [Marlin paper][marlin], which obtains preprocessing zkSNARKs with universal and updatable SRS by combining two ingredients:

* an **algebraic holographic proof**
* a **polynomial commitment scheme**

The first ingredient is provided as part of this library, and is an efficient algebraic holographic proof for R1CS (a generalization of arithmetic circuit satisfiability supported by many argument systems). The second ingredient is imported from [`poly-commit`](https://github.com/scipr-lab/poly-commit). See [the Marlin paper][marlin] for evaluation details.

## Profiling

This library is instrumented with profiling infrastructure that prints detailed traces of execution time. To enable this, compile with `cargo build --features print-trace`.


## Benchmarks

All benchmarks below are performed over the BLS12-381 curve implemented in the [`algebra`](https://github.com/scipr-lab/zexe/tree/master/algebra) library, with the `asm` feature activated. Benchmarks were run on a machine with an Intel Xeon 6136 CPU running at 3.0 GHz.


### Running time compared to Groth16 

The graphs below compare the running time, in single-thread execution, of Marlin's indexer, prover, and verifier algorithms with the corresponding algorithms of [Groth16][groth16] (the state of the art in preprocessing zkSNARKs for R1CS with circuit-specific SRS) as implemented in [`groth16`](https://github.com/scipr-lab/zexe/tree/master/groth16). We evaluate Marlin's algorithms when instantiated with the PC scheme from [[CHMMVW20]][marlin] (denoted "M-AHP w/ PC of [[CHMMVW20]][marlin]"), and the PC scheme from [[MBKM19]][sonic] (denoted "M-AHP w/ PC of [[MBKM19]][sonic]").

<p align="center">
<img hspace="20" src="https://user-images.githubusercontent.com/3220730/82859703-52546100-9ecc-11ea-8f9d-ec2fb10f042d.png" width="45%" alt = "Indexer">
<img hspace="20" src="https://user-images.githubusercontent.com/3220730/82859705-52ecf780-9ecc-11ea-84cc-99eda9f13d6a.png" width="45%" alt = "Prover">
</p>
<p align="center">
<img src="https://user-images.githubusercontent.com/3220730/82859701-52546100-9ecc-11ea-8422-877080662073.png" width="45%" alt = "Verifier">
</p>

### Multi-threaded performance

The following graphs compare the running time of Marlin's prover when instantiated with the PC scheme from [[CHMMVW20]][marlin] (left) and the PC scheme from [[MBKM19]][sonic] (right) when executed with a different number of threads.

<p align="center">
<img hspace="20" src="https://user-images.githubusercontent.com/3220730/82859700-51bbca80-9ecc-11ea-9fe1-53a611693dd1.png" width="45%" alt = "Multi-threaded scaling of Marlin AHP with the PC scheme from [CHMMVW20]">
<img hspace="20" src="https://user-images.githubusercontent.com/3220730/82859698-51233400-9ecc-11ea-8a32-37379116e828.png" width="45%" alt = "Multi-threaded scaling of Marlin AHP with the PC scheme from [MBKM19]">
</p>

### Proof size

We compare the proof size of Marlin with that of [Groth16][groth16]. We instantiate the Marlin SNARK with the PC scheme from [[CHMMVW20]][marlin], and the PC scheme from [[MBKM19]][sonic].

|                   Scheme                   | Proof size in bytes |
|:------------------------------------------:|:---------------------:|
| Marlin AHP with PC of [[CHMMVW20]][marlin] |         880         |
| Marlin AHP with PC of [[MBKM19]][sonic]    |         784         |
|  [\[Groth16\]][groth16]                    |         192         |

[marlin]: https://ia.cr/2019/1047
[sonic]: https://ia.cr/2019/099
[groth16]: https://ia.cr/2016/260

## Reference paper

[Marlin: Preprocessing zkSNARKs with Universal and Updatable SRS][marlin]     
Alessandro Chiesa, Yuncong Hu, Mary Maller, [Pratyush Mishra](https://www.github.com/pratyush), Noah Vesely, [Nicholas Ward](https://www.github.com/npwardberkeley)     
EUROCRYPT 2020
