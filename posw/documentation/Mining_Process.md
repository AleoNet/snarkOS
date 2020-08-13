# The Mining Process

We specify a mining algorithm for the PoSW consensus protocol that is based on modular exponentiation over some group $\mathbb{G}$. We denote by $\mathcal{R}$ the relation representing the PoSW circuit, and set a NIZK tuple $(\mathcal{G}, \mathcal{P}, \mathcal{V})$ to generate the common reference string $\mathbf{crs} = \mathcal{G}(\mathcal{R})$. We are interested in defining an algorithm for $\mathcal{P}$ with a size $S$ precomputation string that minimizes the number of multiplications performed in $\mathbb{G}$.

## Modular MultiExponentiation

Since the PoW process reduces to the hardness of exponentiation, we work in a model where we need to compute $q$ instances of exponentiating $k$ random indices $x_{i,j} \in \mathbb{Z}_p, (i,j) \in [q]\times [k]$ for prime $p$ of size $n = \mathsf{log}(p)$ by some random bases $G_i \in \mathbb{G}$: $$ \mathsf{MultiExp}(\{G_i\}_{i = 1}^k , x_1, ..., x_q) = \left( \prod_{i = 1}^k G_i^{x_{1,i}}, ..., \prod_{i = 1}^k G_i^{x_{q,i}}\right).$$

The algorithm $\mathcal{A} = (\mathcal{A}_1, \mathcal{A}_2)$ proceeds in two stages: first $\mathcal{A}_1$ precomputes a string of $S$ group elements in $\mathbb{G}$ from the common reference string $\mathbf{crs} = \{G_i\}_{i = 1}^k$. $\mathcal{A}_2$ then takes this as input along with $q$ sets of $k$ elements in $\mathbb{Z}_p$ and produces $q$ outputs $\{\pi_i\}_{i = 1}^q$. 

For each of the bases $G_i$, compute $S/k$ exponents and store them as part of the precomputation string. These exponents will be the radix decomposition bases for $\mathbb{Z}_p$ at the maximal permissible depth $c$. On average, for each index we require at most  $n/(3+\mathsf{log}(S) - \mathsf{log}(k)- \log{n})$ multiplications for a total of $q \cdot k \cdot n/(\mathsf{log}(S) - \mathsf{log}(k) - \log{n})$. This means that the size of the precomputation string $S$ grows exponentially with a linear improvement in proving time.

## Security \& Miner Size

For a precomputation table of $S = k \cdot (n/c) \cdot (2^c - 1)$ group elements, each exponentiation can be performed in $n/c$ multiplications on average. However, at some point a maximal $c^*$  is obtained that balances the communication cost of sending more precomputed elements with the cost of performing additional multiplications. We can thus operate under the assumption that miners work at around that level, and look at the security it implies.

We investigate proof generation times for various values of $c \in \mathbb{N}_+$. At constant block frequency, these can be used to project what the minimal table size $S$ is for a predicate involving $k$ exponentiations to achieve sufficiently low quantization error and collision probability. We provide results below for $1$ minute blocks, $64$ byte group elements and a circuit with $k \approx 2^{13}$ exponentiations per proof. Miner size corresponds to the size of its precomputed exponentiation table.

|  Size (GB) | Proof Generation Time (s) | Quantization Error | Collision Probability |                    
| -----------|------------------------------|-------------| -----------|
| 1              |                              |             |
| 2              |                              |             |
| 4              |                              |             |
| 8              |                              |             |


### Security Implications of Hardware Acceleration

Since hardware-accelerated miners would be able to provide order-of-magnitude improvements to the proof generation times for a given table size, the development of faster miners will correspond to a proportional decrease of both the quantization error and collision probabilities felt by the system. This means that incentives are aligned so that as the system grows it provides higher security.