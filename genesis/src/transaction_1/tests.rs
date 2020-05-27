use crate::transaction_1::Transaction1;
use snarkos_models::genesis::Genesis;

#[test]
fn test_transaction_1() {
    let parameters = Transaction1::load_bytes();
    assert_eq!(Transaction1::SIZE, parameters.len() as u64);
}
