# The Mining Process

We specify a mining algorithm for the PoSW consensus protocol that is based on modular exponentiation over some group <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}">. We denote by <img src="https://render.githubusercontent.com/render/math?math=\mathbb{R}"> the relation representing the PoSW circuit, and set a NIZK tuple <img src="https://render.githubusercontent.com/render/math?math=(\mathcal{G}, \mathcal{P}, \mathcal{V})"> to generate the common reference string <img src="https://render.githubusercontent.com/render/math?math=\mathbf{crs} = \mathcal{G}(\mathcal{R})">. We are interested in defining an algorithm for <img src="https://render.githubusercontent.com/render/math?math=\mathcal{P}"> with a size <img src="https://render.githubusercontent.com/render/math?math=S"> precomputation string that minimizes the number of multiplications performed in <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}">.

## Modular MultiExponentiation

Since the PoW process reduces to the hardness of exponentiation, we work in a model where we need to compute <img src="https://render.githubusercontent.com/render/math?math=q"> instances of exponentiating <img src="https://render.githubusercontent.com/render/math?math=k"> random indices <img src="https://render.githubusercontent.com/render/math?math=x_{i,j} \in \mathbb{Z}_p, (i,j) \in [q]\times [k]"> for prime <img src="https://render.githubusercontent.com/render/math?math=p"> of size <img src="https://render.githubusercontent.com/render/math?math=n = \mathsf{log}(p)"> by some random bases <img src="https://render.githubusercontent.com/render/math?math=G_i \in \mathbb{G}">:

<img src="https://render.githubusercontent.com/render/math?math=\mathsf{MultiExp}(\{G_i\}_{i = 1}^k , x_1, ..., x_q) = \left( \prod_{i = 1}^k G_i^{x_{1,i}}, ..., \prod_{i = 1}^k G_i^{x_{q,i}}\right).">

The algorithm <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A} = (\mathcal{A}_1, \mathcal{A}_2)"> proceeds in two stages: first <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A}_1"> precomputes a string of <img src="https://render.githubusercontent.com/render/math?math=S"> group elements in <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}"> from the common reference string <img src="https://render.githubusercontent.com/render/math?math=\mathbf{crs} = \{G_i\}_{i = 1}^k">. <img src="https://render.githubusercontent.com/render/math?math=\mathcal{A}_2"> then takes this as input along with <img src="https://render.githubusercontent.com/render/math?math=q"> sets of <img src="https://render.githubusercontent.com/render/math?math=k"> elements in <img src="https://render.githubusercontent.com/render/math?math=\mathbb{Z}_p"> and produces <img src="https://render.githubusercontent.com/render/math?math=q"> outputs <img src="https://render.githubusercontent.com/render/math?math=\{\pi_i\}_{i = 1}^q">.

## Security \& Miner Size

For a precomputation table of <img src="https://render.githubusercontent.com/render/math?math=S = k \cdot (n/c) \cdot (2^c - 1)"> group elements, each exponentiation can be performed in <img src="https://render.githubusercontent.com/render/math?math=n/c"> multiplications on average. However, at some point a maximal <img src="https://render.githubusercontent.com/render/math?math=c^*">  is obtained that balances the communication cost of sending more precomputed elements with the cost of performing additional multiplications. We can thus operate under the assumption that miners work at around that level, and look at the security it implies.

We fix <img src="https://render.githubusercontent.com/render/math?math=S"> and <img src="https://render.githubusercontent.com/render/math?math=k"> and investigate proof generation times for fixed table size. Proving times in the GM17 and Marlin provers alongside the corresponding quantization error and collision probabilities are provided below for a single-threaded desktop machine. Security is with respect to <img src="https://render.githubusercontent.com/render/math?math=1"> minute blocks and a circuit with <img src="https://render.githubusercontent.com/render/math?math=k \approx 2^{13}"> exponentiations per proof.

|  Proof System | Proof Generation Time (s) | Quantization Error (\%) | Collision Probability (\%)|                    
| -----------|------------------------------|-------------| -----------|
| Marlin              |   4.65                       | 3.82            | 3.87
| GM17              |     0.91                         |    0.758         | 0.76


### Security Implications of Hardware Acceleration

Since hardware-accelerated miners would be able to provide order-of-magnitude improvements to the proof generation times for a given table size, the development of faster miners will correspond to a proportional decrease of both the quantization error and collision probabilities felt by the system. This means that incentives are aligned so that as the system grows it provides higher security.
