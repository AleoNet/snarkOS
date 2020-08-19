# The PoSW Predicate

The predicate used as part of the PoSW circuit verifies the inclusion of transactions in a given block. The various building blocks are defined below alongside the relevant implementation parameters.

## System State

The state of the system is given by a Merkle tree <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_\mathcal{G}(h)"> of depth <img src="https://render.githubusercontent.com/render/math?math=h"> over a CRT function <img src="https://render.githubusercontent.com/render/math?math=\mathcal{G}: \{0,1\}^{k} \rightarrow \{0,1\}^{k/2}">, where <img src="https://render.githubusercontent.com/render/math?math=\mathcal{G}"> is taken to be SHA-256 with k = 512. We denote this as the "state tree". Each leaf is the unique id of a transaction to be processed, and the variable <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}"> is the root of the tree.

<img align="left" src="Binary_tree.png" style="float:right"></img>

The PoSW circuit takes the <img src="https://render.githubusercontent.com/render/math?math=q \leq d"> subtree of the state tree and computes a Merkle tree of depth <img src="https://render.githubusercontent.com/render/math?math=q">. The leaves of the tree are the depth <img src="https://render.githubusercontent.com/render/math?math=q"> elements of the state tree <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_\mathcal{H}(h)">, instantiated over k-bit leaves with a different CRT function <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H} : \{0,1\}^{k} \rightarrow \{0,1\}^{k/2}"> as a new PoSW tree <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_{\mathcal{H}}(q)">. This layout is illustrated in the diagram on the left. 

The circuit implementation for <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}"> masks the witness variables based on a pseudorandom seed, which is part of the predicate statement. This is required to achieve non-amortization guarantees. We set <img src="https://render.githubusercontent.com/render/math?math=q = 3"> throughout.

## Pedersen Primitives
The <img src="https://render.githubusercontent.com/render/math?math=k">-bit Pedersen hash function over <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}"> is a CRT hash given by: 
<img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}^G(x) = \prod_{i = 1}^k G_i^{x_i},"> where <img src="https://render.githubusercontent.com/render/math?math=G_i \in \mathbb{G}"> are randomly sampled generators and <img src="https://render.githubusercontent.com/render/math?math=x_i"> the <img src="https://render.githubusercontent.com/render/math?math=i">-th input bit of <img src="https://render.githubusercontent.com/render/math?math=x">. CRT security of this function reduces to the hardness of the Discrete Logarithm Problem (DLP) over the group <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}.">

The above function can be evaluated in a 'masked' fashion, using the primitives below.

### Symmetric Pedersen Gadget

The <img src="https://render.githubusercontent.com/render/math?math=k">-bit symmetric Pedersen hash is defined with the same security guarantees as <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}: \{0,1\}^k \rightarrow \mathbb{G}"> where: 

<img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}^H_{sym}(\rho) = \prod_{i = 1}^k H_i^{1 - 2\rho_i}."> 

#### Circuit Structure

Define group variables <img src="https://render.githubusercontent.com/render/math?math=Q = (Q_x, Q_y), h_i = (h^i_x, h^i_y) \in (\mathbb{F}_p^2)^k">. Check the following evaluations:
- If <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 0">  set <img src="https://render.githubusercontent.com/render/math?math=h_i = H_i">, else if <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 1"> set to <img src="https://render.githubusercontent.com/render/math?math=h_i = H_i^{-1}">.
- <img src="https://render.githubusercontent.com/render/math?math=Q_0"> is the identity and <img src="https://render.githubusercontent.com/render/math?math=Q_i = Q_{i-1} \cdot h_i">.

This requires <img src="https://render.githubusercontent.com/render/math?math=k"> Edwards multiplications (6 constraints each), and a bit lookup for each of the <img src="https://render.githubusercontent.com/render/math?math=h_i"> in addition to <img src="https://render.githubusercontent.com/render/math?math=k"> booleanity checks.

This is evaluated by ``precomputed_base_symmetric_multiscalar_mul`` in ``PedersenCRHGadget``.
  

### Masked Pedersen Gadget
The <img src="https://render.githubusercontent.com/render/math?math=k">-length masked Pedersen hash function over <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}"> is a CRT hash function <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}_{mask}: \{0,1\}^{k} \times \{0,1\}^k \times \mathbb{G} \rightarrow \mathbb{G}"> given by:

<img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}_{mask}^{G,H}(\rho, x,P) = P \cdot\prod_{i = 1}^k (\mathbb{1}[x_i \oplus \rho_i = 1] G_i^{2x_i - 1} H_i^{2\rho_i -1} + \mathbb{1}[x_i \oplus \rho_i = 0] H_i^{2\rho_i -1})">

Where <img src="https://render.githubusercontent.com/render/math?math=x_i"> and <img src="https://render.githubusercontent.com/render/math?math=\rho_i"> the <img src="https://render.githubusercontent.com/render/math?math=i">-th bits of <img src="https://render.githubusercontent.com/render/math?math=x"> and <img src="https://render.githubusercontent.com/render/math?math=\rho"> respectively, while <img src="https://render.githubusercontent.com/render/math?math=G_i \in \mathbb{G}"> are randomly sampled generators of <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}"> and <img src="https://render.githubusercontent.com/render/math?math=\oplus"> the bitwise XOR operation. The variable <img src="https://render.githubusercontent.com/render/math?math=P \in \mathbb{G}"> is appended as an input as well, for the demasking operation.

#### Circuit Structure
Define group variables <img src="https://render.githubusercontent.com/render/math?math=Q = (Q_x, Q_y), g_i = (g^i_x, g^i_y) \in (\mathbb{F}_p^2)^k"> and boolean variables <img src="https://render.githubusercontent.com/render/math?math=z \in \mathbb{F}_p^k">. Perform the following evaluations:
- With a 2-bit lookup, for all <img src="https://render.githubusercontent.com/render/math?math=i \in [k]"> set <img src="https://render.githubusercontent.com/render/math?math=g_i :=">
 	- <img src="https://render.githubusercontent.com/render/math?math=H_i^{-1}"> if <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 0"> and <img src="https://render.githubusercontent.com/render/math?math=x_i = 0">
 	 - <img src="https://render.githubusercontent.com/render/math?math=H_i"> if <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 1"> and <img src="https://render.githubusercontent.com/render/math?math=x_i = 1">
 	- <img src="https://render.githubusercontent.com/render/math?math=G_i \cdot H_i^{-1}"> if <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 1"> and <img src="https://render.githubusercontent.com/render/math?math=x_i = 0">
 	- <img src="https://render.githubusercontent.com/render/math?math=G_i^{-1} \cdot H_i"> if <img src="https://render.githubusercontent.com/render/math?math=\rho_i = 0"> and <img src="https://render.githubusercontent.com/render/math?math=x_i = 1">
- <img src="https://render.githubusercontent.com/render/math?math=Q_0 = P"> and <img src="https://render.githubusercontent.com/render/math?math=Q_i = Q_{i-1} \cdot g_i">.

This requires <img src="https://render.githubusercontent.com/render/math?math=k"> Edwards multiplications (6 constraints each), a 2-bit lookup for each of the <img src="https://render.githubusercontent.com/render/math?math=g_i"> (2 constraints each) and <img src="https://render.githubusercontent.com/render/math?math=k"> booleanity checks.

This is evaluated by ``precomputed_base_scalar_mul_masked`` in ``PedersenCRHGadget``.

### Pedersen Hash Gadget

We instantiate a circuit verifying <img src="https://render.githubusercontent.com/render/math?math=M"> evaluations of <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}^G"> using circuits for <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}^{G,H}_{mask}"> and <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}^H_{sym}"> over <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}">. Note that elements are variables in <img src="https://render.githubusercontent.com/render/math?math=\mathbb{F}_p">, while pairs of variables <img src="https://render.githubusercontent.com/render/math?math=(z_x,z_y) \in \mathbb{F}_p^2"> are parsed as elliptic curve points in <img src="https://render.githubusercontent.com/render/math?math=\mathbb{G}."> We presume that the <img src="https://render.githubusercontent.com/render/math?math=H_i, G_i \in \mathbb{G}"> have been precomputed and are accessible as constants.

#### Inputs:

The <img src="https://render.githubusercontent.com/render/math?math=k">-length masked evaluation of <img src="https://render.githubusercontent.com/render/math?math=M"> Pedersen hashes takes as inputs:
- For <img src="https://render.githubusercontent.com/render/math?math=i \in [ M ]">, boolean variables <img src="https://render.githubusercontent.com/render/math?math=x^i = \{x^i_1, .., x^i_k\}">. 
- A boolean seed <img src="https://render.githubusercontent.com/render/math?math=\rho \in \{0,1\}^k \subseteq \mathbb{F}^k_p">.
#### Evaluations:
- Set <img src="https://render.githubusercontent.com/render/math?math=z \leftarrow \mathcal{H}^H_{sym}(\rho)">.
-  For all <img src="https://render.githubusercontent.com/render/math?math=i \in [M]">,  set <img src="https://render.githubusercontent.com/render/math?math=(o^x_i, o_i^y) = \mathcal{H}^{G,H}_{mask}(\rho, x^i, z)">.

#### Outputs:
The <img src="https://render.githubusercontent.com/render/math?math=k/2"> length set of variables <img src="https://render.githubusercontent.com/render/math?math=o^{i}_{x}"> as the truncated outputs.

### Instantiation
We use BLS12-377 as the underlying group, which implies an output length of 256+1 = 257 bits (using point-compression) which we truncate to 256 bits. Security reduction to the hardness of ECDLP yields a security level of <img src="https://render.githubusercontent.com/render/math?math=$\lambda \approx 128"> bits. The input length is set to k = 512 bits. 

## PoSW Circuit 

The PoSW tree $<img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_{\mathcal{H}}(q)"> takes in the subroots of the state tree's <img src="https://render.githubusercontent.com/render/math?math=q">-depth nodes as leaves, and uses the <img src="https://render.githubusercontent.com/render/math?math=k">-bit Pedersen hash gadget with respect to a seed parameter <img src="https://render.githubusercontent.com/render/math?math=\rho"> to compute the root <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i">. The seed parameter <img src="https://render.githubusercontent.com/render/math?math=\rho = \mathsf{PRF}(\mathsf{state}_i \| n)"> is the output of a pseudorandom function <img src="https://render.githubusercontent.com/render/math?math=\mathsf{PRF}"> with boolean inputs the nonce <img src="https://render.githubusercontent.com/render/math?math=n"> and the tree root.

### Seed Generation
We generate the seed <img src="https://render.githubusercontent.com/render/math?math=\rho"> in the following way for each predicate:

1. Given input nonce <img src="https://render.githubusercontent.com/render/math?math=n \in \{0,1\}^{256}"> and <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state_i} \in \{0,1\}^{256}$">, compute <img src="https://render.githubusercontent.com/render/math?math=\rho_0 \in \{0,1\}^{256}"> as <img src="https://render.githubusercontent.com/render/math?math=\rho_0 = \mathsf{BLAKE}(n \| \mathsf{state_i})">, where <img src="https://render.githubusercontent.com/render/math?math=\|"> represents string concatenation.

2. If the <img src="https://render.githubusercontent.com/render/math?math=i">-th bit <img src="https://render.githubusercontent.com/render/math?math=\rho_{0,i}"> of <img src="https://render.githubusercontent.com/render/math?math=\rho_0"> is 0 or 1, set the (2i-1)-th and 2i-th bits of <img src="https://render.githubusercontent.com/render/math?math=\rho"> as 10 or 01 respectively. This gives a <img src="https://render.githubusercontent.com/render/math?math=\rho \in \{0,1\}^{512}"> of constant Hamming distance 256.

This is all done outside of the circuit, and is required input format for every valid instance.

### Circuit Size

#### Statement-Witness Definition
A valid statement <img src="https://render.githubusercontent.com/render/math?math=\phi = \langle \mathsf{state}_i, n \rangle \in \{0,1\}^{512} \subset \mathbb{F}_p^{512}">, where: 

1. <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i \in \{0,1\}^{256}"> the bitwise representation of the PoSW root node of the updated state variable.
2. <img src="https://render.githubusercontent.com/render/math?math=n \in \{0,1\}^{256}"> the bitwise representation of the nonce.

The witness <img src="https://render.githubusercontent.com/render/math?math=w"> for the above statement consists of:

1. A boolean representation of <img src="https://render.githubusercontent.com/render/math?math=\rho \in \{0,1\}^{512}">.

2. The subroot leaves <img src="https://render.githubusercontent.com/render/math?math=\{x_i\}_{i = 1}^{2^q}, x_i \in \{0,1\}^{512}"> corresponding to <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i">.

3. Boolean representations of the intermediate node values of <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_{\mathcal{H}}(q)">.

#### Evaluations

For the root <img src="https://render.githubusercontent.com/render/math?math=\mathsf{state}_i"> and all internal nodes of <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_{\mathcal{H}}(q)">, perform a computation of the <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}"> gadget with the node value as output and its children as inputs. 

The PoSW circuit verifies that <img src="https://render.githubusercontent.com/render/math?math=\mathsf{Tree}_{\mathcal{H}}(q)"> is correctly generated. This requires the computation of <img src="https://render.githubusercontent.com/render/math?math=2^{q-1} + 1"> instances of <img src="https://render.githubusercontent.com/render/math?math=\mathcal{H}">.



 
