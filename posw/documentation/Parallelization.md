## Parallelization

The PoSW circuit admits parallelization in each of the bases that need to be exponentiated. This means that parallel hardware can provide much lower proving times, which correspond with higher chain security.  However, this comes at the cost of favoring larger miners who are capable of running large parallel computing instances. If the underlying circuit computation was made to be inherently sequential, then it would fullfil the definition of being a Verifiable Delay Function (VDF).

### Recursion

Recursive computation can be used to convert the PoSW circuit into one performing sequential computation. This is achieved by splitting the circuit into sequentual proofs that 'pass' an intermediate set of witnesses through many sub-circuits: each performing a small amount of computation. Since by recursion each proof is needed as input to the next proof, making each subcircuit small will ensure that the process of generating the final proof is almost completely sequential.

### Towards a VDF Construction

Even though the current design could be adapted to ensure non-parallelizability, this would come at high efficiency costs due to the requirement for recursive computation. Therefore, it is desirable to begin with a *parallelizable* instance for which hardware can be developed since this will provide a meaningful security guarantee (low collision probability due to lower proof generation times). After hardware and cryptographic optimizations have been performed so that recursive composition is efficient enough to use, the protocol can easily transition to a state where the underlying proof generation is inherently sequential. 
