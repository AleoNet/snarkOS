use crate::{block_header::GenesisBlockHeader, transaction_1::Transaction1};
use snarkos_models::genesis::Genesis;
use snarkos_utilities::variable_length_integer::variable_length_integer;

pub struct GenesisBlock;

impl Genesis for GenesisBlock {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 84;

    fn load_bytes() -> Vec<u8> {
        let mut buffer = vec![];

        let block_header_bytes = GenesisBlockHeader::load_bytes();

        let num_transactions: u64 = 1;
        let transaction_1_bytes = Transaction1::load_bytes();

        buffer.extend(block_header_bytes);
        buffer.extend(variable_length_integer(num_transactions));
        buffer.extend(transaction_1_bytes);

        buffer
    }
}
