use snarkos_models::parameters::Parameter;

pub struct GenesisAccount;

impl Parameter for GenesisAccount {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 224;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("genesis_account");
        buffer.to_vec()
    }
}
