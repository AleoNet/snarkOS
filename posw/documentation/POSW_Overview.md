# Proof of Succinct Work
Proof of Succinct Work (PoSW) is a consensus protocol that generates proofs on system validity as a useful byprocess of performing proof-of-work . The encoded predicate verifies the transactions that will be included in a given block update, while ensuring that the underlying proof computation is still a time-lock puzzle.

## Overview

PoSW is a variant of Bitcoin's SHA-based difficulty adjusting algorithm, with the key difference being that the underlying computation is not an arbitrary hash function but rather a proof of knowledge. This allows the PoSW solution to not only act as PoW to ensure system consensus, but also provide verification of transaction inclusion in a given block. We work in the asynchronous model, and presume existence of an honest majority of miners (or provers). 

Given a Non-Interactive Zero-Knowledge proof tuple <img src="https://render.githubusercontent.com/render/math?math=(\mathcal{G},\mathcal{P},\mathcal{V})"> for a given relation <img src="https://render.githubusercontent.com/render/math?math=\mathcal{R}">, the PoSW algorithm consists of the following:

1. Given a set of (valid) transactions <img src="https://render.githubusercontent.com/render/math?math=T_i = \{t_1, ..., t_n\}"> and a current state <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i">, calculate <img src="https://render.githubusercontent.com/render/math?math=\mathsf{NewState}(\mathsf{state}_i, T_i) \rightarrow (\mathsf{state}_{i+1}, w_{i+1})"> where:
	- <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i"> is the state at the <img src="https://render.githubusercontent.com/render/math?math=i">-th block, and
	- <img src="https://render.githubusercontent.com/render/math?math=w_i"> is auxiliary information attesting to the validity of <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_{i+1}.">

2. Sample a random nonce <img src="https://render.githubusercontent.com/render/math?math=n"> and compute: 

<img src="https://render.githubusercontent.com/render/math?math=\mathcal{P}(\mathbf{crs}, \langle n,  \mathsf{state}_{i+1} \rangle, w_{i+1}) \rightarrow \pi_n.">

3. If <img src="https://render.githubusercontent.com/render/math?math=\mathsf{PRF}(\pi_n) \leq d">, set <img src="https://render.githubusercontent.com/render/math?math=n_{i+1} = n"> and <img src="https://render.githubusercontent.com/render/math?math=\pi_{i + 1} = \pi_n">. Return to Step 2 otherwise.

The function <img src="https://render.githubusercontent.com/render/math?math=\mathsf{PRF}"> is a pseudorandom function used to evaluate the difficulty condition, while <img src="https://render.githubusercontent.com/render/math?math=\mathbf{crs}"> is the public output of <img src="https://render.githubusercontent.com/render/math?math=\mathcal{G}">.

### Difficulty Update

The difficulty update procedure is exactly the same as in Bitcoin and other PoW currencies, updating <img src="https://render.githubusercontent.com/render/math?math=d"> adaptively based on network hashrate. It is iteratively updated based on the maximal and current targets every fixed number of blocks and guarantees constant block time.

## Consensus Security

Since PoSW needs to satisfy PoW guarantees, it requires security properties that make it a time-lock puzzle. We identify these below.

### Amortization Resistance

The most important property of any PoW system is non-batchability: computation of many instances of the problem should not provide substantial speed-ups to miners through the reuse of information.

We work in the Generic Group Model (GGM), where miners have access to an oracle <img src="https://render.githubusercontent.com/render/math?math=\mathcal{O}"> performing a given hard computation in the random encoding of some group <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}">. Computational difficulty is then given by the number of oracle queries that a miner makes to <img src="https://render.githubusercontent.com/render/math?math=\mathcal{O}">. In this setup, we can define the notion of <img src="https://render.githubusercontent.com/render/math?math=\epsilon">-*amortization resistance* as the ratio of oracle queries performed by the optimal algorithm <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A}^{\mathcal{O}}_{\mathcal{P}, \ell(n)}"> computing <img src="https://render.githubusercontent.com/render/math?math=\ell(n) = \mathsf{poly}(n)"> proofs simultaneously vs. the algorithm <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A}^{\mathcal{O}}_{\mathcal{P}, 1}"> computing each <img src="https://render.githubusercontent.com/render/math?math=\ell(n)"> proof individually. Here <img src="https://render.githubusercontent.com/render/math?math=n"> is proof size, <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Queries}(\mathcal{A}^{\mathcal{O}})"> the number of queries <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A}^{\mathcal{O}}"> makes to <img src="https://render.githubusercontent.com/render/math?math=\mathcal{O}"> and <img src="https://render.githubusercontent.com/render/math?math=\mathbf{x}_i"> the (randomly sampled) <img src="https://render.githubusercontent.com/render/math?math=i">-th problem instance: 

<img src="https://render.githubusercontent.com/render/math?math=\epsilon \leq 1 - \frac{\mathsf{Queries}(\mathcal{A}^{\mathcal{O}}_{\mathcal{P}, \ell(n)}(\{\mathbf{x_i}\}_{i = 1}^{\ell(n)}))}{\sum_{i = 1}^{\ell(n)} \mathsf{Queries}(\mathcal{A}^{\mathcal{O}}_{\mathcal{P}, 1}(\mathbf{x_i}))}.">

Intuitively, <img src="https://render.githubusercontent.com/render/math?math=\epsilon"> is the advantage that a large miner receives due to the amortizability of the underlying puzzle. If <img src="https://render.githubusercontent.com/render/math?math=\epsilon = 0">, no algorithm attains speedup from batching and the puzzle is *perfectly amortizable*. 

### Quantization Error \& Forking Probability

Unlike other PoW schemes, the repeated underlying computation in PoSW takes substantially more time to compute a potential solution (a single proof) than other puzzles. This is because NIZK proof generation is computationally intensive, which can have an effect on the security of the underlying chain if it's comparable to block generation time.

#### Quantization Error
 In the setting where proof generation time is a significant fraction of the block time, a slow miner who hears of a new solution before finishing their current attempt will have to discard partially-computed proofs to begin mining on the new block. We call the proportion of work wasted due to this the *quantization error* <img src="https://render.githubusercontent.com/render/math?math=\epsilon_Q"> of the protocol, which is equal to: 
 
 <img src="https://render.githubusercontent.com/render/math?math=\epsilon_Q = 1 - \frac{\tau}{e^{\tau} - 1},"> where block time is normalized to <img src="https://render.githubusercontent.com/render/math?math=1"> and average proof generation time is set to <img src="https://render.githubusercontent.com/render/math?math=\tau">.

Note that <img src="https://render.githubusercontent.com/render/math?math=\tau \rightarrow 0"> implies <img src="https://render.githubusercontent.com/render/math?math=\epsilon_Q \rightarrow 0">. In this limit, the work discarded by miners approaches zero, demonstrating that the ratio <img src="https://render.githubusercontent.com/render/math?math=\tau = \tau_p/\tau_b"> between proof generation time <img src="https://render.githubusercontent.com/render/math?math=\tau_p"> and block time <img src="https://render.githubusercontent.com/render/math?math=\tau_b"> should be minimized.

#### Forking Probability

The quantum effects observed above can also increase the number of observed collisions in the protocol. A conservative upper bound on this
can be obtained from a worst-case analysis of synchronized miners with identical proving time <img src="https://render.githubusercontent.com/render/math?math=\tau">, maximizing the probability of simultaneous solutions. If miners aren’t synchronized, they may opt to finish their current effort after a block is found (rather than discard partial work), but even if all miners do so this reduces to the synchronous case above.

The expected number of solutions in a mining “round” is a random variable <img src="https://render.githubusercontent.com/render/math?math=X \sim \mathsf{Po}(\tau)">. We can obtain a bound on the forking probability <img src="https://render.githubusercontent.com/render/math?math=\epsilon_F"> from the ratio of rounds with multiple solutions to rounds with any solution: 

<img src="https://render.githubusercontent.com/render/math?math=\epsilon_F \leq \frac{1 - \mathsf{Poisson}(1, \tau)}{1 - \mathsf{Poisson}(0, \tau)} \leq \frac{\tau}{2},"> where <img src="https://render.githubusercontent.com/render/math?math=f(q) = \mathsf{Poisson}(q,\tau)"> the PDF of <img src="https://render.githubusercontent.com/render/math?math=X">. 

For a fixed block time, this means that any improvements in proving time (due to hardware acceleration and/or circuit size changes) will proportionally decrease this error bound.

## Instantiation

Below we provide the design choices of the PoSW implementation that achieve the above security guarantees, alongside the relevant security parameters.

### Proof System Choice

The choice of proof system is necessary (but not sufficient) in achieving amortization resistance, for the protocol. Although a variety of proof systems can be chosen, there are specific properties that need to be satisfied. PoSW uses the Marlin architecture, which is consistent with the properties below in the non-interactive Random Oracle setting.

#### Simulation Extractability
A simulation-extractable (SE) proof system has a unique encoding for every valid instance-witness pair <img src="https://render.githubusercontent.com/render/math?math=\langle \phi, w\rangle."> This implies that valid proofs cannot be rerandomized without explicit knowledge of a different witness for <img src="https://render.githubusercontent.com/render/math?math=\phi">. Otherwise, an adversarial prover would be able to change the encoding of a proof after computation until it satisfies difficulty, which would violate the non-amortization requirement.

#### Reduction to an Average-Case Hard Problem

A non-amortizable prover should reduce in difficulty to a problem known (or postulated) to be non-batchable (or 'hard') on average. Since the state-of-the-art proof systems are almost all built using Kate commitments, we work in this paradigm and reduce proof computation to the problem of multi-exponentiation of a set of given (random) bases <img src="https://render.githubusercontent.com/render/math?math=\{G_i\}_{i = 1}^m \in \mathbb{G}^m"> by a set of random indices <img src="https://render.githubusercontent.com/render/math?math=\{x_i\}_{i  =1}^m \in \mathbb{Z}_p^m">. In this problem, hardness is measured in the number of queries to a multiplication oracle <img src="https://render.githubusercontent.com/render/math?math=\mathcal{O}_m"> in the given group's encoding.

Although the above problem is *not* non-amortizable in the setting of unbounded space, it can be shown to be non-amortizable on average for miners with a fixed size precomputation string. In addition, we note that the average-case hard instances only differ from the worst-case hard instances by a small amount of computation and will not provide realistic speed-ups in practice: i.e. resampling instances until an 'easier' one is found has bounded benefit.

### Predicate Design

The choice of predicate is also crucial in ensuring the above security guarantees. Below we identify the relevant properties that the computed relation <img src="https://render.githubusercontent.com/render/math?math=\mathcal{R}"> should satisfy:

1. **Usefulness:** The proof is a proof of knowledge for a statement providing inherent value to the protocol. We opt for a relation that verifies the inclusion of a set of transactions in the given block.

2. **Computational Uniqueness:** An adversary cannot find a new witness <img src="https://render.githubusercontent.com/render/math?math=w_2"> for <img src="https://render.githubusercontent.com/render/math?math=\phi"> given knowledge of a valid instance-witness pair <img src="https://render.githubusercontent.com/render/math?math=\langle \phi, w\rangle">. This ensures that the miner cannot resample witnesses for <img src="https://render.githubusercontent.com/render/math?math=\phi"> to reduce computational burden.

3. **Non-Amortization:** Valid witnesses for <img src="https://render.githubusercontent.com/render/math?math=\mathcal{R}"> need to "look" sufficiently random to reduce to the average-case hardness of multiexponentiation. The chosen predicate achieves <img src="https://render.githubusercontent.com/render/math?math=\epsilon = 0"> (or perfect) non-amortization in this context.


### Error Bounds

Fix error bounds for quantization and forking error to 3% and 1.5% respectively. For a protocol with <img src="https://render.githubusercontent.com/render/math?math=1">-minute block times, this implies that average proof generation times need to be upper bounded by <img src="https://render.githubusercontent.com/render/math?math=\tau = 1.8 \approx 2"> seconds.
