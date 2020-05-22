use crate::{consensus::GM17Verifier, ConsensusParameters};
use snarkos_dpc::base_dpc::instantiated::*;

mod e2e;
pub use e2e::*;

mod fixture;
pub use fixture::*;

mod posw;
pub use posw::*;

use once_cell::sync::Lazy;

pub static TEST_CONSENSUS: Lazy<ConsensusParameters<GM17Verifier>> = Lazy::new(|| ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
    verifier: GM17Verifier(POSW_PP.1.clone()),
});

pub struct Wallet {
    pub private_key: &'static str,
    pub address: &'static str,
}

pub const TEST_WALLETS: [Wallet; 3] = [
    Wallet {
        private_key: "b7c666199c254a675f8b440f3077bd6da1addb00be76ba042da208e60fcb8b0783743dd489bd5e4226272f788dceb594a162b1a970f90c908f862c8d19cf2b1208d1410f2ce58fe7b7a3668e051abac8b9708bdce2b17497498bd507447f6d0359cfd686f31fe24cad9b72c1aec7d09d2e8e2ae92e33178297054429615452cc010101010101010101010101010101010101010101010101010101010101010131d19d54b9a2d6a3204afc38b7a84b8f9330c21136480d661d53d84003790600",
        address: "ea986eef993ecc2fea3cf2becbfc4301f265ca7585f2eca953c9832899529e00",
    },
    Wallet {
        private_key: "47d748b4c5659758cb2bf7288d3ba40725dae13d3220266b86ecab9e9402aa0b6301f5befaebc742b1bdfc265ff51fca26488c5f8a90244536069e4b436f7d03f0f2af9e6dddd4dcaa7f8efa9576f457e354521bd3f8dd6048ec1bf8d4424502afa94a86fef3ddb314b9545bc33071eb9a270a1f7294d530809f9d664d2f3b4d0202020202020202020202020202020202020202020202020202020202020202eac67cce8c505e47755f0f2b2f4698d15c9fb15b33d34419b506a1b97e291d01",
        address: "097fefc6f9be4afd4203ed1cf50c7bb0afc6ea4d6f9b589d666271b1064a1d01",
    },
    Wallet {
        private_key: "1a4ad98f81ac936b35705b478042a48509fb8230fc9f92a823a63a446a7ef210f123d1dad7084ad0cdeb18c39da01a5df9795f642d19e6e452f641a1a8ba310ab7a1efdebc1ddf1c3fd665dc57f856c142829cf1123593c7ac8a77acaa79db01aaca7ac2a2f5dd722899dcbb84b62f8087f0e6a7e725ff11ecc12dfe452c9d9c03030303030303030303030303030303030303030303030303030303030303030bb0f7dd75c085bacdd188d3902c8b34cdd375b53240022bd925c27319351102",
        address: "b63caa4c9368bf9f2de493f61a9dbe8a555fb7dc9a56464b2b251eaf6cb2ca07",
    },
];
